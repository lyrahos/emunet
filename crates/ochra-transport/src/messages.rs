//! All message payload structs for the Ochra wire protocol.
//!
//! Each message type defined in the Ochra v5.5 specification has a corresponding
//! struct here. These structs are serialized to CBOR for inclusion in
//! [`ProtocolMessage`](crate::wire::ProtocolMessage) envelopes.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Message type constants (Section 4.3)
// ---------------------------------------------------------------------------

/// Message type for capability exchange (0x0001).
pub const MSG_CAPABILITY_EXCHANGE: u16 = 0x0001;
/// Message type for ping (0x0002).
pub const MSG_PING: u16 = 0x0002;
/// Message type for pong (0x0003).
pub const MSG_PONG: u16 = 0x0003;
/// Message type for goodbye (0x0004).
pub const MSG_GOODBYE: u16 = 0x0004;

/// Message type for chunk request (0x0010).
pub const MSG_CHUNK_REQUEST: u16 = 0x0010;
/// Message type for chunk response (0x0011).
pub const MSG_CHUNK_RESPONSE: u16 = 0x0011;
/// Message type for chunk advertise (0x0012).
pub const MSG_CHUNK_ADVERTISE: u16 = 0x0012;
/// Message type for service receipt acknowledgement (0x0013).
pub const MSG_SERVICE_RECEIPT_ACK: u16 = 0x0013;

/// Message type for DHT get (0x0020).
pub const MSG_DHT_GET: u16 = 0x0020;
/// Message type for DHT get response (0x0021).
pub const MSG_DHT_GET_RESPONSE: u16 = 0x0021;
/// Message type for DHT put (0x0022).
pub const MSG_DHT_PUT: u16 = 0x0022;
/// Message type for DHT put response (0x0023).
pub const MSG_DHT_PUT_RESPONSE: u16 = 0x0023;
/// Message type for DHT find node (0x0024).
pub const MSG_DHT_FIND_NODE: u16 = 0x0024;
/// Message type for DHT find node response (0x0025).
pub const MSG_DHT_FIND_NODE_RESPONSE: u16 = 0x0025;

/// Message type for establish introduction (0x0030).
pub const MSG_ESTABLISH_INTRO: u16 = 0x0030;
/// Message type for establish intro ack (0x0031).
pub const MSG_ESTABLISH_INTRO_ACK: u16 = 0x0031;
/// Message type for introduce1 (0x0032).
pub const MSG_INTRODUCE1: u16 = 0x0032;
/// Message type for introduce2 (0x0033).
pub const MSG_INTRODUCE2: u16 = 0x0033;
/// Message type for rendezvous join (0x0034).
pub const MSG_RENDEZVOUS_JOIN: u16 = 0x0034;
/// Message type for rendezvous joined (0x0035).
pub const MSG_RENDEZVOUS_JOINED: u16 = 0x0035;
/// Message type for rendezvous relay (0x0036).
pub const MSG_RENDEZVOUS_RELAY: u16 = 0x0036;
/// Message type for rendezvous teardown (0x0037).
pub const MSG_RENDEZVOUS_TEARDOWN: u16 = 0x0037;

/// Message type for MLS welcome (0x0040).
pub const MSG_MLS_WELCOME: u16 = 0x0040;
/// Message type for MLS commit (0x0041).
pub const MSG_MLS_COMMIT: u16 = 0x0041;
/// Message type for MLS application (0x0042).
pub const MSG_MLS_APPLICATION: u16 = 0x0042;
/// Message type for MLS proposal (0x0043).
pub const MSG_MLS_PROPOSAL: u16 = 0x0043;
/// Message type for MLS key package (0x0044).
pub const MSG_MLS_KEY_PACKAGE: u16 = 0x0044;

/// Message type for FROST DKG round1 (0x0050).
pub const MSG_FROST_DKG_ROUND1: u16 = 0x0050;
/// Message type for FROST DKG round2 (0x0051).
pub const MSG_FROST_DKG_ROUND2: u16 = 0x0051;
/// Message type for FROST sign request (0x0052).
pub const MSG_FROST_SIGN_REQUEST: u16 = 0x0052;
/// Message type for FROST sign share (0x0053).
pub const MSG_FROST_SIGN_SHARE: u16 = 0x0053;
/// Message type for quorum proposal (0x0054).
pub const MSG_QUORUM_PROPOSAL: u16 = 0x0054;
/// Message type for quorum vote (0x0055).
pub const MSG_QUORUM_VOTE: u16 = 0x0055;
/// Message type for quorum result (0x0056).
pub const MSG_QUORUM_RESULT: u16 = 0x0056;

/// Message type for gossip publish (0x0060).
pub const MSG_GOSSIP_PUBLISH: u16 = 0x0060;
/// Message type for gossip forward (0x0061).
pub const MSG_GOSSIP_FORWARD: u16 = 0x0061;
/// Message type for gossip prune (0x0062).
pub const MSG_GOSSIP_PRUNE: u16 = 0x0062;

/// Message type for whisper send (0x0070).
pub const MSG_WHISPER_SEND: u16 = 0x0070;
/// Message type for whisper deliver (0x0071).
pub const MSG_WHISPER_DELIVER: u16 = 0x0071;
/// Message type for whisper ack (0x0072).
pub const MSG_WHISPER_ACK: u16 = 0x0072;

/// Message type for oracle request (0x0080).
pub const MSG_ORACLE_REQUEST: u16 = 0x0080;
/// Message type for oracle response (0x0081).
pub const MSG_ORACLE_RESPONSE: u16 = 0x0081;
/// Message type for oracle attestation (0x0082).
pub const MSG_ORACLE_ATTESTATION: u16 = 0x0082;

/// Message type for recovery request (0x0090).
pub const MSG_RECOVERY_REQUEST: u16 = 0x0090;
/// Message type for recovery response (0x0091).
pub const MSG_RECOVERY_RESPONSE: u16 = 0x0091;
/// Message type for recovery share (0x0092).
pub const MSG_RECOVERY_SHARE: u16 = 0x0092;
/// Message type for recovery complete (0x0093).
pub const MSG_RECOVERY_COMPLETE: u16 = 0x0093;

// ---------------------------------------------------------------------------
// 0x0001 Capability Exchange
// ---------------------------------------------------------------------------

/// Capability exchange payload, sent immediately after QUIC connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityExchange {
    /// Protocol version (must be 5).
    pub protocol_version: u8,
    /// Node ID (BLAKE3 hash of the PIK public key).
    pub node_id: [u8; 32],
    /// Bitmask of supported features.
    pub features: u64,
    /// Human-readable agent string (e.g. "ochra-daemon/0.1.0").
    pub agent: String,
    /// Supported message types the peer is willing to handle.
    pub supported_messages: Vec<u16>,
}

// ---------------------------------------------------------------------------
// 0x0002-0x0003 Ping / Pong
// ---------------------------------------------------------------------------

/// Ping payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ping {
    /// Random 8-byte nonce to be echoed in the pong.
    pub nonce: [u8; 8],
}

/// Pong payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pong {
    /// Echo of the ping nonce.
    pub nonce: [u8; 8],
}

// ---------------------------------------------------------------------------
// 0x0004 Goodbye
// ---------------------------------------------------------------------------

/// Goodbye reason codes.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GoodbyeReason {
    /// Normal shutdown.
    Normal = 0,
    /// Protocol violation detected.
    ProtocolViolation = 1,
    /// Timeout.
    Timeout = 2,
    /// Too many connections.
    TooManyConnections = 3,
    /// Authentication failure.
    AuthFailure = 4,
    /// Application-defined reason.
    Other = 255,
}

impl GoodbyeReason {
    /// Convert a raw byte to a `GoodbyeReason`.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Normal,
            1 => Self::ProtocolViolation,
            2 => Self::Timeout,
            3 => Self::TooManyConnections,
            4 => Self::AuthFailure,
            _ => Self::Other,
        }
    }
}

/// Goodbye payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Goodbye {
    /// Reason code for disconnection.
    pub reason: u8,
    /// Optional human-readable detail.
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// 0x0010-0x0013 Chunk messages
// ---------------------------------------------------------------------------

/// Chunk request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkRequest {
    /// BLAKE3 hash of the requested chunk.
    pub chunk_hash: [u8; 32],
    /// Offset within the chunk (for resumption).
    pub offset: u64,
    /// Maximum bytes to return.
    pub max_length: u32,
}

/// Chunk response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkResponse {
    /// BLAKE3 hash of the chunk being served.
    pub chunk_hash: [u8; 32],
    /// Offset corresponding to the request.
    pub offset: u64,
    /// The chunk data (or portion thereof).
    pub data: Vec<u8>,
    /// Total chunk size in bytes.
    pub total_size: u64,
}

/// Chunk advertise payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkAdvertise {
    /// List of chunk hashes this node is willing to serve.
    pub chunk_hashes: Vec<[u8; 32]>,
    /// TTL in seconds for this advertisement.
    pub ttl_secs: u32,
}

/// Service receipt acknowledgement payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceReceiptAck {
    /// BLAKE3 hash of the chunk that was served.
    pub chunk_hash: [u8; 32],
    /// Bytes received so far.
    pub bytes_received: u64,
    /// Ed25519 signature from the requester acknowledging receipt.
    pub ack_signature: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 0x0020-0x0025 DHT messages
// ---------------------------------------------------------------------------

/// DHT get request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtGet {
    /// The key to look up in the DHT.
    pub key: [u8; 32],
}

/// DHT get response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtGetResponse {
    /// The requested key.
    pub key: [u8; 32],
    /// The value, if found.
    pub value: Option<Vec<u8>>,
    /// Nodes closer to the key for iterative routing.
    pub closer_nodes: Vec<DhtNodeInfo>,
}

/// DHT put request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtPut {
    /// The key to store.
    pub key: [u8; 32],
    /// The value to store.
    pub value: Vec<u8>,
    /// TTL in seconds for the stored record.
    pub ttl_secs: u32,
    /// Ed25519 signature over key || value.
    pub signature: Vec<u8>,
}

/// DHT put response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtPutResponse {
    /// The key that was stored.
    pub key: [u8; 32],
    /// Whether the put was accepted.
    pub accepted: bool,
}

/// DHT find-node request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtFindNode {
    /// Target node ID to search for.
    pub target: [u8; 32],
}

/// DHT find-node response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtFindNodeResponse {
    /// Target that was searched for.
    pub target: [u8; 32],
    /// Nodes closest to the target.
    pub nodes: Vec<DhtNodeInfo>,
}

/// Information about a DHT node for routing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtNodeInfo {
    /// The node ID (BLAKE3 hash of PIK public key).
    pub node_id: [u8; 32],
    /// Socket address of the node ("ip:port").
    pub addr: String,
}

// ---------------------------------------------------------------------------
// 0x0030-0x0037 Rendezvous messages
// ---------------------------------------------------------------------------

/// Establish introduction point payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EstablishIntro {
    /// Unique identifier for this introduction point session.
    pub intro_id: [u8; 16],
    /// X25519 public key for the hidden service.
    pub service_x25519_pk: [u8; 32],
    /// Ed25519 signature authorizing this intro point.
    pub auth_signature: Vec<u8>,
}

/// Establish introduction point acknowledgement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EstablishIntroAck {
    /// Echoed intro session ID.
    pub intro_id: [u8; 16],
    /// Whether the intro point was accepted.
    pub accepted: bool,
}

/// Introduce1 payload (client to intro point).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Introduce1 {
    /// Intro session ID targeting the hidden service.
    pub intro_id: [u8; 16],
    /// Client's ephemeral X25519 public key.
    pub client_x25519_pk: [u8; 32],
    /// Encrypted payload for the hidden service (onion-layered).
    pub encrypted_payload: Vec<u8>,
}

/// Introduce2 payload (intro point to hidden service).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Introduce2 {
    /// Intro session ID.
    pub intro_id: [u8; 16],
    /// Client's ephemeral X25519 public key (forwarded from Introduce1).
    pub client_x25519_pk: [u8; 32],
    /// Encrypted payload (forwarded from Introduce1).
    pub encrypted_payload: Vec<u8>,
}

/// Rendezvous join payload (client establishes a rendezvous point).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousJoin {
    /// Unique rendezvous cookie.
    pub rendezvous_cookie: [u8; 16],
}

/// Rendezvous joined payload (rendezvous point confirms join).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousJoined {
    /// Echoed rendezvous cookie.
    pub rendezvous_cookie: [u8; 16],
    /// Whether the join was successful.
    pub success: bool,
}

/// Rendezvous relay payload (data relayed through rendezvous point).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousRelay {
    /// Rendezvous cookie identifying the session.
    pub rendezvous_cookie: [u8; 16],
    /// Encrypted data payload.
    pub data: Vec<u8>,
}

/// Rendezvous teardown payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousTeardown {
    /// Rendezvous cookie identifying the session to tear down.
    pub rendezvous_cookie: [u8; 16],
}

// ---------------------------------------------------------------------------
// 0x0040-0x0044 MLS messages
// ---------------------------------------------------------------------------

/// MLS welcome message payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsWelcome {
    /// Target group ID.
    pub group_id: [u8; 32],
    /// Serialized MLS Welcome message.
    pub welcome_data: Vec<u8>,
}

/// MLS commit message payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsCommit {
    /// Target group ID.
    pub group_id: [u8; 32],
    /// MLS epoch this commit advances from.
    pub epoch: u64,
    /// Serialized MLS Commit message.
    pub commit_data: Vec<u8>,
}

/// MLS application message payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsApplication {
    /// Target group ID.
    pub group_id: [u8; 32],
    /// MLS epoch.
    pub epoch: u64,
    /// Encrypted MLS application data.
    pub ciphertext: Vec<u8>,
}

/// MLS proposal message payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsProposal {
    /// Target group ID.
    pub group_id: [u8; 32],
    /// MLS epoch.
    pub epoch: u64,
    /// Serialized MLS Proposal message.
    pub proposal_data: Vec<u8>,
}

/// MLS key package payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsKeyPackage {
    /// The node ID this key package belongs to.
    pub node_id: [u8; 32],
    /// Serialized MLS KeyPackage.
    pub key_package_data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 0x0050-0x0056 FROST / Quorum messages
// ---------------------------------------------------------------------------

/// FROST DKG round 1 package payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrostDkgRound1 {
    /// Session identifier for the DKG ceremony.
    pub session_id: [u8; 16],
    /// Participant identifier within the DKG.
    pub participant_id: u16,
    /// Serialized round 1 package.
    pub package_data: Vec<u8>,
}

/// FROST DKG round 2 package payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrostDkgRound2 {
    /// Session identifier for the DKG ceremony.
    pub session_id: [u8; 16],
    /// Sender participant identifier.
    pub sender_id: u16,
    /// Receiver participant identifier.
    pub receiver_id: u16,
    /// Serialized round 2 package.
    pub package_data: Vec<u8>,
}

/// FROST sign request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrostSignRequest {
    /// Session identifier for the signing ceremony.
    pub session_id: [u8; 16],
    /// Message hash to be signed.
    pub message_hash: [u8; 32],
    /// Serialized signing commitments.
    pub commitments_data: Vec<u8>,
}

/// FROST sign share payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrostSignShare {
    /// Session identifier for the signing ceremony.
    pub session_id: [u8; 16],
    /// Participant identifier.
    pub participant_id: u16,
    /// Serialized signature share.
    pub share_data: Vec<u8>,
}

/// Quorum proposal payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumProposal {
    /// Unique proposal identifier.
    pub proposal_id: [u8; 16],
    /// The epoch this proposal pertains to.
    pub epoch: u32,
    /// CBOR-encoded proposal body.
    pub body: Vec<u8>,
    /// Ed25519 signature from the proposer.
    pub proposer_signature: Vec<u8>,
}

/// Quorum vote payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumVote {
    /// Proposal identifier being voted on.
    pub proposal_id: [u8; 16],
    /// Whether this is an approval vote.
    pub approve: bool,
    /// Voter's node ID.
    pub voter_node_id: [u8; 32],
    /// Ed25519 signature from the voter.
    pub voter_signature: Vec<u8>,
}

/// Quorum result payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumResult {
    /// Proposal identifier.
    pub proposal_id: [u8; 16],
    /// Whether the proposal was accepted.
    pub accepted: bool,
    /// FROST aggregate signature from the quorum.
    pub quorum_signature: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 0x0060-0x0062 Gossip messages
// ---------------------------------------------------------------------------

/// Gossip publish payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GossipPublish {
    /// Topic identifier.
    pub topic: [u8; 32],
    /// Gossip message data.
    pub data: Vec<u8>,
    /// Hop count (decremented at each relay).
    pub ttl: u8,
    /// Unique message ID to prevent re-broadcast.
    pub gossip_msg_id: [u8; 16],
}

/// Gossip forward payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GossipForward {
    /// Topic identifier.
    pub topic: [u8; 32],
    /// Gossip message data.
    pub data: Vec<u8>,
    /// Remaining TTL.
    pub ttl: u8,
    /// Unique message ID.
    pub gossip_msg_id: [u8; 16],
}

/// Gossip prune payload (instructs a peer to stop relaying a topic).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GossipPrune {
    /// Topic to prune.
    pub topic: [u8; 32],
    /// Reason for prune.
    pub reason: u8,
}

// ---------------------------------------------------------------------------
// 0x0070-0x0072 Whisper messages
// ---------------------------------------------------------------------------

/// Whisper send payload (end-to-end encrypted direct message).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperSend {
    /// Recipient's Whisper session ID.
    pub session_id: [u8; 16],
    /// Double-ratchet encrypted ciphertext.
    pub ciphertext: Vec<u8>,
    /// Sender's current ratchet public key.
    pub ratchet_pk: [u8; 32],
    /// Message counter in the current ratchet chain.
    pub counter: u32,
    /// Previous chain length (for out-of-order handling).
    pub previous_chain_length: u32,
}

/// Whisper deliver payload (from relay to recipient).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperDeliver {
    /// Session ID.
    pub session_id: [u8; 16],
    /// Encrypted ciphertext (forwarded from WhisperSend).
    pub ciphertext: Vec<u8>,
    /// Sender's ratchet public key.
    pub ratchet_pk: [u8; 32],
    /// Message counter.
    pub counter: u32,
    /// Previous chain length.
    pub previous_chain_length: u32,
}

/// Whisper acknowledgement payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperAck {
    /// Session ID.
    pub session_id: [u8; 16],
    /// Counter of the message being acknowledged.
    pub acked_counter: u32,
}

// ---------------------------------------------------------------------------
// 0x0080-0x0082 Oracle messages
// ---------------------------------------------------------------------------

/// Oracle request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleRequest {
    /// Request identifier.
    pub request_id: [u8; 16],
    /// Type of oracle query (application-defined).
    pub query_type: u16,
    /// CBOR-encoded query parameters.
    pub params: Vec<u8>,
}

/// Oracle response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleResponse {
    /// Request identifier (echoed from request).
    pub request_id: [u8; 16],
    /// Whether the oracle accepted the query.
    pub success: bool,
    /// Response data.
    pub data: Vec<u8>,
    /// Ed25519 signature from the oracle node.
    pub oracle_signature: Vec<u8>,
}

/// Oracle attestation payload (threshold-signed oracle result).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleAttestation {
    /// Request identifier.
    pub request_id: [u8; 16],
    /// Attested data.
    pub data: Vec<u8>,
    /// FROST aggregate signature from the oracle quorum.
    pub quorum_signature: Vec<u8>,
    /// Epoch at which attestation was made.
    pub epoch: u32,
}

// ---------------------------------------------------------------------------
// 0x0090-0x0093 Recovery messages
// ---------------------------------------------------------------------------

/// Recovery request payload (initiating social recovery).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryRequest {
    /// The node ID of the PIK being recovered.
    pub target_node_id: [u8; 32],
    /// Recovery session identifier.
    pub recovery_session_id: [u8; 16],
    /// New X25519 public key for the recovered identity.
    pub new_x25519_pk: [u8; 32],
}

/// Recovery response payload (guardian acknowledging recovery request).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryResponse {
    /// Recovery session identifier.
    pub recovery_session_id: [u8; 16],
    /// Guardian's node ID.
    pub guardian_node_id: [u8; 32],
    /// Whether the guardian accepted the recovery.
    pub accepted: bool,
}

/// Recovery share payload (guardian providing their share).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryShare {
    /// Recovery session identifier.
    pub recovery_session_id: [u8; 16],
    /// Guardian's node ID.
    pub guardian_node_id: [u8; 32],
    /// Encrypted recovery share data.
    pub encrypted_share: Vec<u8>,
}

/// Recovery complete payload (recovery has been completed).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryComplete {
    /// Recovery session identifier.
    pub recovery_session_id: [u8; 16],
    /// Whether recovery was successful.
    pub success: bool,
    /// New PIK public key hash (if recovery succeeded).
    pub new_pik_hash: Option<[u8; 32]>,
}

// ---------------------------------------------------------------------------
// Typed message enum
// ---------------------------------------------------------------------------

/// Strongly-typed enum of all protocol message payloads.
///
/// This enum provides a convenient way to match on message types after
/// deserializing the CBOR payload from a [`ProtocolMessage`](crate::wire::ProtocolMessage).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TypedMessage {
    /// Capability exchange (0x0001).
    CapabilityExchange(CapabilityExchange),
    /// Ping (0x0002).
    Ping(Ping),
    /// Pong (0x0003).
    Pong(Pong),
    /// Goodbye (0x0004).
    Goodbye(Goodbye),

    /// Chunk request (0x0010).
    ChunkRequest(ChunkRequest),
    /// Chunk response (0x0011).
    ChunkResponse(ChunkResponse),
    /// Chunk advertise (0x0012).
    ChunkAdvertise(ChunkAdvertise),
    /// Service receipt ack (0x0013).
    ServiceReceiptAck(ServiceReceiptAck),

    /// DHT get (0x0020).
    DhtGet(DhtGet),
    /// DHT get response (0x0021).
    DhtGetResponse(DhtGetResponse),
    /// DHT put (0x0022).
    DhtPut(DhtPut),
    /// DHT put response (0x0023).
    DhtPutResponse(DhtPutResponse),
    /// DHT find node (0x0024).
    DhtFindNode(DhtFindNode),
    /// DHT find node response (0x0025).
    DhtFindNodeResponse(DhtFindNodeResponse),

    /// Establish intro (0x0030).
    EstablishIntro(EstablishIntro),
    /// Establish intro ack (0x0031).
    EstablishIntroAck(EstablishIntroAck),
    /// Introduce1 (0x0032).
    Introduce1(Introduce1),
    /// Introduce2 (0x0033).
    Introduce2(Introduce2),
    /// Rendezvous join (0x0034).
    RendezvousJoin(RendezvousJoin),
    /// Rendezvous joined (0x0035).
    RendezvousJoined(RendezvousJoined),
    /// Rendezvous relay (0x0036).
    RendezvousRelay(RendezvousRelay),
    /// Rendezvous teardown (0x0037).
    RendezvousTeardown(RendezvousTeardown),

    /// MLS welcome (0x0040).
    MlsWelcome(MlsWelcome),
    /// MLS commit (0x0041).
    MlsCommit(MlsCommit),
    /// MLS application (0x0042).
    MlsApplication(MlsApplication),
    /// MLS proposal (0x0043).
    MlsProposal(MlsProposal),
    /// MLS key package (0x0044).
    MlsKeyPackage(MlsKeyPackage),

    /// FROST DKG round 1 (0x0050).
    FrostDkgRound1(FrostDkgRound1),
    /// FROST DKG round 2 (0x0051).
    FrostDkgRound2(FrostDkgRound2),
    /// FROST sign request (0x0052).
    FrostSignRequest(FrostSignRequest),
    /// FROST sign share (0x0053).
    FrostSignShare(FrostSignShare),
    /// Quorum proposal (0x0054).
    QuorumProposal(QuorumProposal),
    /// Quorum vote (0x0055).
    QuorumVote(QuorumVote),
    /// Quorum result (0x0056).
    QuorumResult(QuorumResult),

    /// Gossip publish (0x0060).
    GossipPublish(GossipPublish),
    /// Gossip forward (0x0061).
    GossipForward(GossipForward),
    /// Gossip prune (0x0062).
    GossipPrune(GossipPrune),

    /// Whisper send (0x0070).
    WhisperSend(WhisperSend),
    /// Whisper deliver (0x0071).
    WhisperDeliver(WhisperDeliver),
    /// Whisper ack (0x0072).
    WhisperAck(WhisperAck),

    /// Oracle request (0x0080).
    OracleRequest(OracleRequest),
    /// Oracle response (0x0081).
    OracleResponse(OracleResponse),
    /// Oracle attestation (0x0082).
    OracleAttestation(OracleAttestation),

    /// Recovery request (0x0090).
    RecoveryRequest(RecoveryRequest),
    /// Recovery response (0x0091).
    RecoveryResponse(RecoveryResponse),
    /// Recovery share (0x0092).
    RecoveryShare(RecoveryShare),
    /// Recovery complete (0x0093).
    RecoveryComplete(RecoveryComplete),
}

impl TypedMessage {
    /// Return the wire-protocol message type code for this message.
    pub fn msg_type(&self) -> u16 {
        match self {
            Self::CapabilityExchange(_) => MSG_CAPABILITY_EXCHANGE,
            Self::Ping(_) => MSG_PING,
            Self::Pong(_) => MSG_PONG,
            Self::Goodbye(_) => MSG_GOODBYE,
            Self::ChunkRequest(_) => MSG_CHUNK_REQUEST,
            Self::ChunkResponse(_) => MSG_CHUNK_RESPONSE,
            Self::ChunkAdvertise(_) => MSG_CHUNK_ADVERTISE,
            Self::ServiceReceiptAck(_) => MSG_SERVICE_RECEIPT_ACK,
            Self::DhtGet(_) => MSG_DHT_GET,
            Self::DhtGetResponse(_) => MSG_DHT_GET_RESPONSE,
            Self::DhtPut(_) => MSG_DHT_PUT,
            Self::DhtPutResponse(_) => MSG_DHT_PUT_RESPONSE,
            Self::DhtFindNode(_) => MSG_DHT_FIND_NODE,
            Self::DhtFindNodeResponse(_) => MSG_DHT_FIND_NODE_RESPONSE,
            Self::EstablishIntro(_) => MSG_ESTABLISH_INTRO,
            Self::EstablishIntroAck(_) => MSG_ESTABLISH_INTRO_ACK,
            Self::Introduce1(_) => MSG_INTRODUCE1,
            Self::Introduce2(_) => MSG_INTRODUCE2,
            Self::RendezvousJoin(_) => MSG_RENDEZVOUS_JOIN,
            Self::RendezvousJoined(_) => MSG_RENDEZVOUS_JOINED,
            Self::RendezvousRelay(_) => MSG_RENDEZVOUS_RELAY,
            Self::RendezvousTeardown(_) => MSG_RENDEZVOUS_TEARDOWN,
            Self::MlsWelcome(_) => MSG_MLS_WELCOME,
            Self::MlsCommit(_) => MSG_MLS_COMMIT,
            Self::MlsApplication(_) => MSG_MLS_APPLICATION,
            Self::MlsProposal(_) => MSG_MLS_PROPOSAL,
            Self::MlsKeyPackage(_) => MSG_MLS_KEY_PACKAGE,
            Self::FrostDkgRound1(_) => MSG_FROST_DKG_ROUND1,
            Self::FrostDkgRound2(_) => MSG_FROST_DKG_ROUND2,
            Self::FrostSignRequest(_) => MSG_FROST_SIGN_REQUEST,
            Self::FrostSignShare(_) => MSG_FROST_SIGN_SHARE,
            Self::QuorumProposal(_) => MSG_QUORUM_PROPOSAL,
            Self::QuorumVote(_) => MSG_QUORUM_VOTE,
            Self::QuorumResult(_) => MSG_QUORUM_RESULT,
            Self::GossipPublish(_) => MSG_GOSSIP_PUBLISH,
            Self::GossipForward(_) => MSG_GOSSIP_FORWARD,
            Self::GossipPrune(_) => MSG_GOSSIP_PRUNE,
            Self::WhisperSend(_) => MSG_WHISPER_SEND,
            Self::WhisperDeliver(_) => MSG_WHISPER_DELIVER,
            Self::WhisperAck(_) => MSG_WHISPER_ACK,
            Self::OracleRequest(_) => MSG_ORACLE_REQUEST,
            Self::OracleResponse(_) => MSG_ORACLE_RESPONSE,
            Self::OracleAttestation(_) => MSG_ORACLE_ATTESTATION,
            Self::RecoveryRequest(_) => MSG_RECOVERY_REQUEST,
            Self::RecoveryResponse(_) => MSG_RECOVERY_RESPONSE,
            Self::RecoveryShare(_) => MSG_RECOVERY_SHARE,
            Self::RecoveryComplete(_) => MSG_RECOVERY_COMPLETE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_serialize_roundtrip() {
        let ping = Ping {
            nonce: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let json = serde_json::to_string(&ping).expect("serialize");
        let restored: Ping = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ping.nonce, restored.nonce);
    }

    #[test]
    fn test_goodbye_reason_from_u8() {
        assert_eq!(GoodbyeReason::from_u8(0), GoodbyeReason::Normal);
        assert_eq!(GoodbyeReason::from_u8(1), GoodbyeReason::ProtocolViolation);
        assert_eq!(GoodbyeReason::from_u8(2), GoodbyeReason::Timeout);
        assert_eq!(GoodbyeReason::from_u8(3), GoodbyeReason::TooManyConnections);
        assert_eq!(GoodbyeReason::from_u8(4), GoodbyeReason::AuthFailure);
        assert_eq!(GoodbyeReason::from_u8(99), GoodbyeReason::Other);
        assert_eq!(GoodbyeReason::from_u8(255), GoodbyeReason::Other);
    }

    #[test]
    fn test_typed_message_msg_type() {
        let ping = TypedMessage::Ping(Ping { nonce: [0; 8] });
        assert_eq!(ping.msg_type(), MSG_PING);

        let pong = TypedMessage::Pong(Pong { nonce: [0; 8] });
        assert_eq!(pong.msg_type(), MSG_PONG);

        let goodbye = TypedMessage::Goodbye(Goodbye {
            reason: 0,
            detail: None,
        });
        assert_eq!(goodbye.msg_type(), MSG_GOODBYE);
    }

    #[test]
    fn test_capability_exchange_serialize() {
        let cap = CapabilityExchange {
            protocol_version: 5,
            node_id: [0xAA; 32],
            features: 0x0001,
            agent: "ochra-test/0.1".to_string(),
            supported_messages: vec![MSG_PING, MSG_PONG, MSG_GOODBYE],
        };
        let json = serde_json::to_string(&cap).expect("serialize");
        let restored: CapabilityExchange = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.protocol_version, 5);
        assert_eq!(restored.agent, "ochra-test/0.1");
        assert_eq!(restored.supported_messages.len(), 3);
    }

    #[test]
    fn test_dht_node_info_serialize() {
        let info = DhtNodeInfo {
            node_id: [0xBB; 32],
            addr: "127.0.0.1:9735".to_string(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let restored: DhtNodeInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.addr, "127.0.0.1:9735");
    }
}
