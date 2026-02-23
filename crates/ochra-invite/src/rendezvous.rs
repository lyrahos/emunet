//! Anonymous rendezvous protocol for introduction points.
//!
//! The rendezvous protocol allows two parties to establish a communication
//! channel without either party revealing their network location to the other.
//!
//! ## Flow
//!
//! 1. **Introduction Point Establishment**: A node selects one or more relays
//!    as introduction points and publishes their info (e.g., in a handle
//!    descriptor or contact exchange token).
//!
//! 2. **Rendezvous Request**: The initiator creates a rendezvous circuit to a
//!    rendezvous point, sends the rendezvous cookie, then contacts the
//!    responder's introduction point with the cookie and rendezvous point info.
//!
//! 3. **Rendezvous Join**: The responder builds a circuit to the rendezvous
//!    point using the cookie, completing the end-to-end circuit.
//!
//! ## Introduction Points
//!
//! Each introduction point is a relay that holds an auth key for forwarding
//! introduction requests. The node publishes `IntroPoint` structs in its
//! handle descriptor or contact exchange token.

use serde::{Deserialize, Serialize};

use crate::{InviteDescriptor, InviteError, Result, SealedInvite};

/// An introduction point entry, published in handle descriptors or
/// contact exchange tokens.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntroPoint {
    /// Node ID of the relay acting as the introduction point.
    pub node_id: [u8; 32],
    /// Auth key for this introduction point (used to encrypt intro requests).
    pub auth_key: [u8; 32],
    /// X25519 public key of the introduction point relay.
    pub relay_x25519_pk: [u8; 32],
}

/// State of an introduction point from the service's perspective.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntroPointState {
    /// The introduction point is being established.
    Establishing,
    /// The introduction point is active and accepting introductions.
    Active,
    /// The introduction point has been retired.
    Retired,
    /// The introduction point failed.
    Failed,
}

/// Manages introduction points for the local node.
///
/// A node typically maintains 2-3 introduction points for redundancy.
pub struct IntroPointManager {
    /// Active introduction points.
    points: Vec<ManagedIntroPoint>,
    /// Maximum number of concurrent introduction points.
    max_points: usize,
}

/// An introduction point with management metadata.
#[derive(Clone, Debug)]
struct ManagedIntroPoint {
    /// The introduction point info.
    point: IntroPoint,
    /// Current state.
    state: IntroPointState,
    /// Number of introductions received through this point.
    #[allow(dead_code)]
    intro_count: u64,
}

impl IntroPointManager {
    /// Create a new introduction point manager.
    ///
    /// # Arguments
    ///
    /// * `max_points` - Maximum concurrent introduction points (typically 2-3)
    pub fn new(max_points: usize) -> Self {
        Self {
            points: Vec::with_capacity(max_points),
            max_points,
        }
    }

    /// Establish a new introduction point at the given relay.
    ///
    /// Generates an auth key for the introduction point and registers it.
    pub fn establish(
        &mut self,
        relay_node_id: [u8; 32],
        relay_x25519_pk: [u8; 32],
    ) -> Result<IntroPoint> {
        if self.active_count() >= self.max_points {
            return Err(InviteError::Malformed(format!(
                "maximum introduction points ({}) already established",
                self.max_points,
            )));
        }

        // Generate a random auth key for this introduction point.
        let mut auth_key = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut auth_key);

        let point = IntroPoint {
            node_id: relay_node_id,
            auth_key,
            relay_x25519_pk,
        };

        self.points.push(ManagedIntroPoint {
            point: point.clone(),
            state: IntroPointState::Active,
            intro_count: 0,
        });

        Ok(point)
    }

    /// Retire an introduction point by node ID.
    pub fn retire(&mut self, node_id: &[u8; 32]) {
        for managed in &mut self.points {
            if managed.point.node_id == *node_id {
                managed.state = IntroPointState::Retired;
            }
        }
    }

    /// Mark an introduction point as failed.
    pub fn mark_failed(&mut self, node_id: &[u8; 32]) {
        for managed in &mut self.points {
            if managed.point.node_id == *node_id {
                managed.state = IntroPointState::Failed;
            }
        }
    }

    /// Record an introduction received through a point.
    pub fn record_introduction(&mut self, node_id: &[u8; 32]) {
        for managed in &mut self.points {
            if managed.point.node_id == *node_id && managed.state == IntroPointState::Active {
                managed.intro_count += 1;
            }
        }
    }

    /// Return all active introduction points.
    pub fn active_points(&self) -> Vec<&IntroPoint> {
        self.points
            .iter()
            .filter(|m| m.state == IntroPointState::Active)
            .map(|m| &m.point)
            .collect()
    }

    /// Return the number of active introduction points.
    pub fn active_count(&self) -> usize {
        self.points
            .iter()
            .filter(|m| m.state == IntroPointState::Active)
            .count()
    }

    /// Remove retired and failed introduction points.
    pub fn cleanup(&mut self) {
        self.points.retain(|m| {
            m.state == IntroPointState::Active || m.state == IntroPointState::Establishing
        });
    }
}

/// A rendezvous point address derived from an invite descriptor.
#[derive(Clone, Debug)]
pub struct RendezvousAddr {
    /// The 32-byte DHT address.
    pub addr: [u8; 32],
}

impl RendezvousAddr {
    /// Derive from an invite descriptor.
    pub fn from_descriptor(descriptor: &InviteDescriptor) -> Self {
        Self {
            addr: descriptor.rendezvous_addr(),
        }
    }

    /// Return the raw address bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.addr
    }
}

/// A rendezvous message wrapping a sealed invite for DHT storage.
#[derive(Clone, Debug)]
pub struct RendezvousMessage {
    /// The rendezvous address (DHT key).
    pub addr: RendezvousAddr,
    /// The sealed invite ciphertext.
    pub ciphertext: Vec<u8>,
}

impl RendezvousMessage {
    /// Create a rendezvous message from a sealed invite.
    pub fn from_sealed(sealed: &SealedInvite) -> Self {
        Self {
            addr: RendezvousAddr {
                addr: sealed.rendezvous_addr,
            },
            ciphertext: sealed.ciphertext.clone(),
        }
    }
}

/// A rendezvous cookie used to match initiator and responder at the
/// rendezvous point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousCookie {
    /// 20-byte random cookie for matching.
    pub cookie: [u8; 20],
}

impl RendezvousCookie {
    /// Generate a new random rendezvous cookie.
    pub fn generate() -> Self {
        let mut cookie = [0u8; 20];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut cookie);
        Self { cookie }
    }

    /// Create from raw bytes.
    pub fn from_bytes(cookie: [u8; 20]) -> Self {
        Self { cookie }
    }
}

/// A rendezvous request sent to the responder's introduction point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousRequest {
    /// The rendezvous cookie for matching.
    pub cookie: RendezvousCookie,
    /// The rendezvous point's node ID.
    pub rendezvous_node_id: [u8; 32],
    /// Encrypted handshake data for the responder.
    pub encrypted_handshake: Vec<u8>,
}

/// The responder's reply, sent to the rendezvous point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RendezvousResponse {
    /// The rendezvous cookie for matching.
    pub cookie: RendezvousCookie,
    /// Encrypted handshake data for the initiator.
    pub encrypted_handshake: Vec<u8>,
}

/// State of a rendezvous flow from the initiator's perspective.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RendezvousState {
    /// Waiting for the responder to join at the rendezvous point.
    WaitingForResponse,
    /// The responder has joined; the circuit is established.
    Established,
    /// The rendezvous timed out.
    TimedOut,
    /// The rendezvous failed.
    Failed,
}

/// Manages the initiator side of a rendezvous flow.
pub struct RendezvousFlow {
    /// The rendezvous cookie.
    cookie: RendezvousCookie,
    /// The rendezvous point's node ID.
    rendezvous_node_id: [u8; 32],
    /// Current state of the flow.
    state: RendezvousState,
}

impl RendezvousFlow {
    /// Create a new rendezvous flow.
    ///
    /// # Arguments
    ///
    /// * `rendezvous_node_id` - Node ID of the relay acting as rendezvous point
    pub fn new(rendezvous_node_id: [u8; 32]) -> Self {
        Self {
            cookie: RendezvousCookie::generate(),
            rendezvous_node_id,
            state: RendezvousState::WaitingForResponse,
        }
    }

    /// Return the rendezvous cookie.
    pub fn cookie(&self) -> &RendezvousCookie {
        &self.cookie
    }

    /// Return the rendezvous point node ID.
    pub fn rendezvous_node_id(&self) -> &[u8; 32] {
        &self.rendezvous_node_id
    }

    /// Return the current state.
    pub fn state(&self) -> &RendezvousState {
        &self.state
    }

    /// Mark the rendezvous as established (responder has joined).
    pub fn mark_established(&mut self) {
        self.state = RendezvousState::Established;
    }

    /// Mark the rendezvous as timed out.
    pub fn mark_timed_out(&mut self) {
        self.state = RendezvousState::TimedOut;
    }

    /// Mark the rendezvous as failed.
    pub fn mark_failed(&mut self) {
        self.state = RendezvousState::Failed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intro_point_manager_establish() {
        let mut mgr = IntroPointManager::new(3);
        let point = mgr
            .establish([0x01u8; 32], [0x02u8; 32])
            .expect("establish");
        assert_eq!(point.node_id, [0x01u8; 32]);
        assert_eq!(point.relay_x25519_pk, [0x02u8; 32]);
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_intro_point_manager_max_points() {
        let mut mgr = IntroPointManager::new(2);
        mgr.establish([0x01u8; 32], [0x10u8; 32]).expect("1");
        mgr.establish([0x02u8; 32], [0x20u8; 32]).expect("2");
        let result = mgr.establish([0x03u8; 32], [0x30u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_intro_point_retire() {
        let mut mgr = IntroPointManager::new(3);
        mgr.establish([0x01u8; 32], [0x10u8; 32])
            .expect("establish");
        assert_eq!(mgr.active_count(), 1);

        mgr.retire(&[0x01u8; 32]);
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_intro_point_mark_failed() {
        let mut mgr = IntroPointManager::new(3);
        mgr.establish([0x01u8; 32], [0x10u8; 32])
            .expect("establish");
        mgr.mark_failed(&[0x01u8; 32]);
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_intro_point_cleanup() {
        let mut mgr = IntroPointManager::new(3);
        mgr.establish([0x01u8; 32], [0x10u8; 32]).expect("1");
        mgr.establish([0x02u8; 32], [0x20u8; 32]).expect("2");
        mgr.retire(&[0x01u8; 32]);
        mgr.cleanup();
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_intro_point_record_introduction() {
        let mut mgr = IntroPointManager::new(3);
        mgr.establish([0x01u8; 32], [0x10u8; 32])
            .expect("establish");
        mgr.record_introduction(&[0x01u8; 32]);
        mgr.record_introduction(&[0x01u8; 32]);
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_rendezvous_addr_from_descriptor() {
        let descriptor = InviteDescriptor::from_secret([0x42u8; 32]);
        let addr = RendezvousAddr::from_descriptor(&descriptor);
        assert_eq!(*addr.as_bytes(), descriptor.rendezvous_addr());
    }

    #[test]
    fn test_rendezvous_cookie_unique() {
        let c1 = RendezvousCookie::generate();
        let c2 = RendezvousCookie::generate();
        assert_ne!(c1.cookie, c2.cookie);
    }

    #[test]
    fn test_rendezvous_cookie_from_bytes() {
        let bytes = [0xAAu8; 20];
        let cookie = RendezvousCookie::from_bytes(bytes);
        assert_eq!(cookie.cookie, bytes);
    }

    #[test]
    fn test_rendezvous_flow_lifecycle() {
        let mut flow = RendezvousFlow::new([0x01u8; 32]);
        assert_eq!(*flow.state(), RendezvousState::WaitingForResponse);
        assert_eq!(*flow.rendezvous_node_id(), [0x01u8; 32]);

        flow.mark_established();
        assert_eq!(*flow.state(), RendezvousState::Established);
    }

    #[test]
    fn test_rendezvous_flow_timeout() {
        let mut flow = RendezvousFlow::new([0x01u8; 32]);
        flow.mark_timed_out();
        assert_eq!(*flow.state(), RendezvousState::TimedOut);
    }

    #[test]
    fn test_rendezvous_flow_failure() {
        let mut flow = RendezvousFlow::new([0x01u8; 32]);
        flow.mark_failed();
        assert_eq!(*flow.state(), RendezvousState::Failed);
    }

    #[test]
    fn test_rendezvous_message_from_sealed() {
        let descriptor = InviteDescriptor::from_secret([0x42u8; 32]);
        let sealed = SealedInvite {
            rendezvous_addr: descriptor.rendezvous_addr(),
            ciphertext: vec![0xBBu8; 64],
        };
        let msg = RendezvousMessage::from_sealed(&sealed);
        assert_eq!(msg.addr.addr, sealed.rendezvous_addr);
        assert_eq!(msg.ciphertext, sealed.ciphertext);
    }

    #[test]
    fn test_rendezvous_request() {
        let cookie = RendezvousCookie::generate();
        let req = RendezvousRequest {
            cookie: cookie.clone(),
            rendezvous_node_id: [0x01u8; 32],
            encrypted_handshake: vec![0xBBu8; 64],
        };
        assert_eq!(req.cookie.cookie, cookie.cookie);
    }
}
