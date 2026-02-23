//! QUIC/TLS 1.3 connection management for the Ochra P2P network.
//!
//! This module provides a QUIC transport layer using [`quinn`] with TLS 1.3.
//! For v1, nodes use self-signed TLS certificates; authentication is deferred
//! to the PIK exchange that occurs over the established QUIC connection.
//!
//! ## ALPN
//!
//! The ALPN protocol identifier is `ochra/5`, corresponding to protocol version 5.
//!
//! ## Connection lifecycle
//!
//! 1. The server binds a QUIC endpoint and listens for incoming connections.
//! 2. The client connects to the server using a client endpoint.
//! 3. After QUIC handshake, both sides exchange [`CapabilityExchange`](crate::messages::CapabilityExchange)
//!    messages on a bidirectional stream.
//! 4. Subsequent messages use additional bidirectional streams.

use std::net::SocketAddr;
use std::sync::Arc;

use quinn::{ClientConfig, Connection, Endpoint, Incoming, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::TransportError;

/// ALPN protocol identifier for Ochra protocol version 5.
pub const ALPN_OCHRA_V5: &[u8] = b"ochra/5";

/// Default QUIC idle timeout in milliseconds.
pub const DEFAULT_IDLE_TIMEOUT_MS: u32 = 30_000;

/// Default maximum number of concurrent bidirectional streams.
pub const DEFAULT_MAX_BI_STREAMS: u32 = 128;

/// Configuration for a QUIC node (server + client combined for P2P).
#[derive(Clone)]
pub struct QuicConfig {
    /// Local address to bind to.
    pub bind_addr: SocketAddr,
    /// Maximum idle timeout in milliseconds.
    pub idle_timeout_ms: u32,
    /// Maximum concurrent bidirectional streams per connection.
    pub max_bi_streams: u32,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], 0)),
            idle_timeout_ms: DEFAULT_IDLE_TIMEOUT_MS,
            max_bi_streams: DEFAULT_MAX_BI_STREAMS,
        }
    }
}

/// A QUIC node that can both listen for and initiate connections.
///
/// In the Ochra P2P network, every node acts as both a client and a server.
/// The `QuicNode` wraps a single Quinn [`Endpoint`] configured for both roles.
pub struct QuicNode {
    /// The underlying Quinn endpoint.
    endpoint: Endpoint,
    /// The local address this node is bound to.
    local_addr: SocketAddr,
}

impl QuicNode {
    /// Create a new QUIC node bound to the configured address.
    ///
    /// Generates a self-signed TLS certificate for the server side.
    /// The certificate is not used for identity verification in v1; PIK exchange
    /// handles authentication after the QUIC connection is established.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Tls`] if TLS configuration fails.
    /// Returns [`TransportError::Io`] if the socket cannot be bound.
    pub fn new(config: QuicConfig) -> Result<Self, TransportError> {
        let (server_config, _cert_der) =
            build_server_config(config.idle_timeout_ms, config.max_bi_streams)?;
        let client_config = build_client_config()?;

        let mut endpoint = Endpoint::server(server_config, config.bind_addr)
            .map_err(|e| TransportError::Io(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        let local_addr = endpoint
            .local_addr()
            .map_err(|e| TransportError::Io(e.to_string()))?;

        tracing::info!(%local_addr, "QUIC node started");

        Ok(Self {
            endpoint,
            local_addr,
        })
    }

    /// Get the local socket address this node is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept the next incoming QUIC connection.
    ///
    /// Returns `None` if the endpoint has been closed.
    pub async fn accept(&self) -> Option<Incoming> {
        self.endpoint.accept().await
    }

    /// Initiate a QUIC connection to a remote peer.
    ///
    /// The `server_name` is used for TLS SNI. For v1 self-signed certificates,
    /// this can be any string (e.g., "ochra-node").
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Connection`] if the connection cannot be established.
    pub async fn connect(
        &self,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<Connection, TransportError> {
        let connecting = self
            .endpoint
            .connect(addr, server_name)
            .map_err(|e| TransportError::Connection(e.to_string()))?;

        let connection = connecting
            .await
            .map_err(|e| TransportError::Connection(e.to_string()))?;

        tracing::debug!(
            remote = %connection.remote_address(),
            "QUIC connection established"
        );

        Ok(connection)
    }

    /// Open a new bidirectional stream on an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Connection`] if the stream cannot be opened.
    pub async fn open_bi(
        connection: &Connection,
    ) -> Result<(SendStream, RecvStream), TransportError> {
        connection
            .open_bi()
            .await
            .map_err(|e| TransportError::Connection(e.to_string()))
    }

    /// Accept the next bidirectional stream on an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Connection`] if the connection is closed.
    pub async fn accept_bi(
        connection: &Connection,
    ) -> Result<(SendStream, RecvStream), TransportError> {
        connection
            .accept_bi()
            .await
            .map_err(|e| TransportError::Connection(e.to_string()))
    }

    /// Send a complete message (length-prefixed) on a send stream.
    ///
    /// Wire format: `[length:4 LE][data:length]`
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Io`] if the write fails.
    pub async fn send_message(stream: &mut SendStream, data: &[u8]) -> Result<(), TransportError> {
        let len = u32::try_from(data.len()).map_err(|_| {
            TransportError::InvalidPacket("message too large for 4-byte length prefix".to_string())
        })?;
        stream
            .write_all(&len.to_le_bytes())
            .await
            .map_err(|e| TransportError::Io(e.to_string()))?;
        stream
            .write_all(data)
            .await
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }

    /// Receive a complete message (length-prefixed) from a receive stream.
    ///
    /// Wire format: `[length:4 LE][data:length]`
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Io`] if the read fails.
    /// Returns [`TransportError::InvalidPacket`] if the length exceeds the maximum.
    pub async fn recv_message(
        stream: &mut RecvStream,
        max_size: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| TransportError::Io(e.to_string()))?;
        let len = u32::from_le_bytes(len_buf) as usize;

        if len > max_size {
            return Err(TransportError::InvalidPacket(format!(
                "message length {len} exceeds maximum {max_size}"
            )));
        }

        let mut buf = vec![0u8; len];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(buf)
    }

    /// Gracefully close the endpoint.
    ///
    /// All active connections will be closed with the given error code and reason.
    pub fn close(&self, error_code: u32, reason: &[u8]) {
        self.endpoint
            .close(quinn::VarInt::from_u32(error_code), reason);
    }

    /// Get a reference to the underlying Quinn endpoint.
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}

// ---------------------------------------------------------------------------
// TLS / certificate helpers
// ---------------------------------------------------------------------------

/// Generate a self-signed TLS certificate and private key for QUIC.
///
/// Returns the DER-encoded certificate and private key.
///
/// # Errors
///
/// Returns [`TransportError::Tls`] if certificate generation fails.
fn generate_self_signed_cert(
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), TransportError> {
    // Generate an Ed25519 keypair; the algorithm is determined by the KeyPair.
    let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519)
        .map_err(|e| TransportError::Tls(format!("key generation failed: {e}")))?;

    let params = rcgen::CertificateParams::new(vec!["ochra-node".to_string()])
        .map_err(|e| TransportError::Tls(format!("cert params failed: {e}")))?;

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| TransportError::Tls(format!("self-signed cert generation failed: {e}")))?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    Ok((cert_der, key_der))
}

/// Build a Quinn [`ServerConfig`] with self-signed TLS and the Ochra ALPN.
///
/// # Errors
///
/// Returns [`TransportError::Tls`] if TLS configuration fails.
fn build_server_config(
    idle_timeout_ms: u32,
    max_bi_streams: u32,
) -> Result<(ServerConfig, CertificateDer<'static>), TransportError> {
    let (cert_der, key_der) = generate_self_signed_cert()?;

    let provider = rustls::crypto::ring::default_provider();
    let mut tls_config = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| TransportError::Tls(format!("server TLS version config failed: {e}")))?
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der)
        .map_err(|e| TransportError::Tls(format!("server TLS config failed: {e}")))?;

    tls_config.alpn_protocols = vec![ALPN_OCHRA_V5.to_vec()];

    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(
        quinn::IdleTimeout::try_from(std::time::Duration::from_millis(u64::from(idle_timeout_ms)))
            .map_err(|e| TransportError::Tls(format!("idle timeout config failed: {e}")))?,
    ));
    transport.max_concurrent_bidi_streams(quinn::VarInt::from_u32(max_bi_streams));

    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)
            .map_err(|e| TransportError::Tls(format!("QUIC server crypto config failed: {e}")))?,
    ));
    server_config.transport_config(Arc::new(transport));

    Ok((server_config, cert_der))
}

/// Build a Quinn [`ClientConfig`] that accepts any server certificate (self-signed).
///
/// In v1, TLS is used only for transport encryption. Node identity is verified
/// through PIK exchange after the QUIC connection is established.
///
/// # Errors
///
/// Returns [`TransportError::Tls`] if TLS configuration fails.
fn build_client_config() -> Result<ClientConfig, TransportError> {
    let provider = rustls::crypto::ring::default_provider();
    let mut tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| TransportError::Tls(format!("client TLS version config failed: {e}")))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    tls_config.alpn_protocols = vec![ALPN_OCHRA_V5.to_vec()];

    let client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)
            .map_err(|e| TransportError::Tls(format!("QUIC client crypto config failed: {e}")))?,
    ));

    Ok(client_config)
}

/// TLS certificate verifier that accepts any server certificate.
///
/// This is intentionally insecure at the TLS level. In the Ochra protocol,
/// node authentication is performed via PIK exchange after the QUIC
/// connection is established. TLS is used solely for transport encryption.
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpn_value() {
        assert_eq!(ALPN_OCHRA_V5, b"ochra/5");
    }

    #[test]
    fn test_default_config() {
        let config = QuicConfig::default();
        assert_eq!(config.idle_timeout_ms, DEFAULT_IDLE_TIMEOUT_MS);
        assert_eq!(config.max_bi_streams, DEFAULT_MAX_BI_STREAMS);
    }

    #[test]
    fn test_generate_self_signed_cert() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok());
        let (cert, key) = result.expect("cert generation");
        assert!(!cert.is_empty());
        match &key {
            PrivateKeyDer::Pkcs8(k) => assert!(!k.secret_pkcs8_der().is_empty()),
            _ => unreachable!("expected PKCS8 key"),
        }
    }

    #[test]
    fn test_build_server_config_succeeds() {
        let result = build_server_config(DEFAULT_IDLE_TIMEOUT_MS, DEFAULT_MAX_BI_STREAMS);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_config_succeeds() {
        let result = build_client_config();
        assert!(result.is_ok());
    }
}
