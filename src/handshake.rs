use crate::payload::Payload;
use crate::trit::Trit;

/// An agent capability advertised during handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// Capability name (e.g., "ternary-search", "strategy-sharing").
    pub name: String,
    /// Capability version.
    pub version: u32,
}

impl Capability {
    pub fn new(name: impl Into<String>, version: u32) -> Self {
        Capability { name: name.into(), version }
    }
}

/// The state of a handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    /// No handshake initiated.
    Idle,
    /// Sent our hello, waiting for peer response.
    HelloSent,
    /// Handshake completed successfully.
    Completed,
    /// Handshake rejected.
    Rejected,
}

/// Errors during handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    /// The peer's protocol version is incompatible.
    VersionMismatch { ours: (u32, u32), theirs: (u32, u32) },
    /// No common capabilities.
    NoCommonCapabilities,
    /// Handshake was already completed or rejected.
    AlreadyFinished,
}

/// A handshake between two agents for capability negotiation.
#[derive(Debug, Clone)]
pub struct Handshake {
    /// Our agent ID.
    pub agent_id: u64,
    /// Our capabilities.
    pub capabilities: Vec<Capability>,
    /// Our min/max protocol version (min, max).
    pub version_range: (u32, u32),
    /// Current state.
    pub state: HandshakeState,
    /// Negotiated common capabilities (set after completion).
    pub negotiated: Vec<Capability>,
    /// Negotiated protocol version (set after completion).
    pub negotiated_version: Option<u32>,
}

impl Handshake {
    /// Create a new handshake initiator.
    pub fn new(agent_id: u64, capabilities: Vec<Capability>, version_range: (u32, u32)) -> Self {
        Handshake {
            agent_id,
            capabilities,
            version_range,
            state: HandshakeState::Idle,
            negotiated: Vec::new(),
            negotiated_version: None,
        }
    }

    /// Create a hello message payload for initiating a handshake.
    pub fn create_hello(&mut self) -> Payload {
        self.state = HandshakeState::HelloSent;
        let mut trits = Vec::new();
        // Encode: agent_id as trits (simplified: each byte -> 6 trits using balanced ternary)
        let id_bytes = self.agent_id.to_be_bytes();
        for &b in &id_bytes {
            let t = byte_to_trits(b);
            trits.extend_from_slice(&t);
        }
        // Version range
        trits.extend_from_slice(&byte_to_trits(self.version_range.0 as u8));
        trits.extend_from_slice(&byte_to_trits(self.version_range.1 as u8));
        // Number of capabilities
        trits.extend_from_slice(&byte_to_trits(self.capabilities.len() as u8));
        // Each capability: name length + name chars + version
        for cap in &self.capabilities {
            trits.extend_from_slice(&byte_to_trits(cap.name.len() as u8));
            for c in cap.name.bytes() {
                trits.extend_from_slice(&byte_to_trits(c));
            }
            trits.extend_from_slice(&byte_to_trits(cap.version as u8));
        }
        Payload::from_trits(&trits)
    }

    /// Process a hello from a peer and produce an ack.
    /// Returns the negotiated capabilities and version, or an error.
    pub fn respond_to_hello(&mut self, hello: &Payload) -> Result<Vec<Capability>, HandshakeError> {
        let trits = hello.trits();
        let mut offset = 0;

        // Skip peer agent ID (8 bytes * 5 trits)
        offset += 8 * 6;

        // Read version range
        let their_min = trits_to_byte(&trits[offset..offset + 6]) as u32;
        offset += 6;
        let their_max = trits_to_byte(&trits[offset..offset + 6]) as u32;
        offset += 6;

        // Check version compatibility
        let negotiated_version = {
            let our_min = self.version_range.0;
            let our_max = self.version_range.1;
            let overlap_start = our_min.max(their_min);
            let overlap_end = our_max.min(their_max);
            if overlap_start > overlap_end {
                return Err(HandshakeError::VersionMismatch {
                    ours: (our_min, our_max),
                    theirs: (their_min, their_max),
                });
            }
            overlap_end // Use highest common version
        };

        // Read peer capabilities
        let num_caps = trits_to_byte(&trits[offset..offset + 6]) as usize;
        offset += 6;
        let mut peer_caps = Vec::new();
        for _ in 0..num_caps {
            let name_len = trits_to_byte(&trits[offset..offset + 6]) as usize;
            offset += 6;
            let name: String = (0..name_len)
                .map(|_| {
                    let b = trits_to_byte(&trits[offset..offset + 6]);
                    offset += 6;
                    b as char
                })
                .collect();
            let version = trits_to_byte(&trits[offset..offset + 6]) as u32;
            offset += 6;
            peer_caps.push(Capability::new(name, version));
        }

        // Negotiate: keep capabilities we share (by name), use min version
        let mut negotiated = Vec::new();
        for our_cap in &self.capabilities {
            if let Some(peer_cap) = peer_caps.iter().find(|c| c.name == our_cap.name) {
                let min_ver = our_cap.version.min(peer_cap.version);
                negotiated.push(Capability::new(&our_cap.name, min_ver));
            }
        }

        if negotiated.is_empty() {
            return Err(HandshakeError::NoCommonCapabilities);
        }

        self.negotiated = negotiated.clone();
        self.negotiated_version = Some(negotiated_version);
        self.state = HandshakeState::Completed;
        Ok(negotiated)
    }

    /// Complete the handshake from the initiator side after receiving ack data.
    pub fn complete(&mut self, negotiated: Vec<Capability>, version: u32) {
        self.negotiated = negotiated;
        self.negotiated_version = Some(version);
        self.state = HandshakeState::Completed;
    }
}

/// Convert a byte to 6 balanced-ternary trits (range -364..364, covers 0..255).
/// We use 6 trits for full range.
fn byte_to_trits(b: u8) -> [Trit; 6] {
    let mut val = b as i32;
    let mut trits = [Trit::Zero; 6];
    for i in (0..6).rev() {
        match val % 3 {
            0 => { trits[i] = Trit::Zero; val /= 3; }
            1 => { trits[i] = Trit::Pos; val = (val - 1) / 3; }
            2 => { trits[i] = Trit::Neg; val = (val + 1) / 3; }
            _ => unreachable!(),
        }
    }
    trits
}

/// Convert 6 balanced-ternary trits back to a byte.
fn trits_to_byte(trits: &[Trit]) -> u8 {
    let mut val: i32 = 0;
    for &t in trits.iter().take(6) {
        val = val * 3 + t.to_i8() as i32;
    }
    val as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_trit_roundtrip() {
        for b in 0u8..=255 {
            let trits = byte_to_trits(b);
            let back = trits_to_byte(&trits);
            assert_eq!(b, back, "roundtrip failed for byte {}", b);
        }
    }

    #[test]
    fn handshake_success() {
        let alice_caps = vec![Capability::new("search", 2), Capability::new("share", 1)];
        let bob_caps = vec![Capability::new("search", 3), Capability::new("share", 1)];
        let mut alice = Handshake::new(1, alice_caps, (1, 3));
        let mut bob = Handshake::new(2, bob_caps, (2, 4));

        let hello = alice.create_hello();
        let result = bob.respond_to_hello(&hello).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "search");
        assert_eq!(result[0].version, 2); // min(2,3)
        assert_eq!(result[1].name, "share");
        assert_eq!(result[1].version, 1);
        assert_eq!(bob.negotiated_version, Some(3)); // min(3,4)
    }

    #[test]
    fn handshake_version_mismatch() {
        let mut alice = Handshake::new(1, vec![Capability::new("search", 1)], (1, 2));
        let mut bob = Handshake::new(2, vec![Capability::new("search", 1)], (5, 7));
        let hello = alice.create_hello();
        let result = bob.respond_to_hello(&hello);
        assert!(matches!(result, Err(HandshakeError::VersionMismatch { .. })));
    }

    #[test]
    fn handshake_no_common_caps() {
        let mut alice = Handshake::new(1, vec![Capability::new("search", 1)], (1, 5));
        let mut bob = Handshake::new(2, vec![Capability::new("compute", 1)], (1, 5));
        let hello = alice.create_hello();
        let result = bob.respond_to_hello(&hello);
        assert!(matches!(result, Err(HandshakeError::NoCommonCapabilities)));
    }
}
