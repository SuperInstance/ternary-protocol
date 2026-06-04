use crate::payload::Payload;

/// Unique message identifier (u64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId(pub u64);

impl MessageId {
    /// Create a new message ID from a u64.
    pub fn new(v: u64) -> Self {
        MessageId(v)
    }

    /// Monotonically incrementing counter-based ID generator.
    pub fn generator(start: u64) -> impl FnMut() -> MessageId {
        let mut counter = start;
        move || {
            let id = MessageId(counter);
            counter += 1;
            id
        }
    }
}

/// A structured message between ternary agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TernaryMessage {
    pub id: MessageId,
    pub sender: u64,
    pub receiver: u64,
    pub payload: Payload,
    pub timestamp: u64,
}

impl TernaryMessage {
    /// Create a new message.
    pub fn new(id: MessageId, sender: u64, receiver: u64, payload: Payload, timestamp: u64) -> Self {
        TernaryMessage { id, sender, receiver, payload, timestamp }
    }

    /// Create a broadcast message (receiver = 0 means all).
    pub fn broadcast(id: MessageId, sender: u64, payload: Payload, timestamp: u64) -> Self {
        TernaryMessage { id, sender, receiver: 0, payload, timestamp }
    }

    /// Is this a broadcast message?
    pub fn is_broadcast(&self) -> bool {
        self.receiver == 0
    }

    /// Encode message to bytes.
    ///
    /// Wire format:
    /// [8B id] [8B sender] [8B receiver] [8B timestamp] [payload bytes]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.id.0.to_be_bytes());
        out.extend_from_slice(&self.sender.to_be_bytes());
        out.extend_from_slice(&self.receiver.to_be_bytes());
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.payload.to_bytes());
        out
    }

    /// Decode message from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 32 {
            return Err("message too short: need at least 32 bytes for header".into());
        }
        let id = MessageId(u64::from_be_bytes(data[0..8].try_into().unwrap()));
        let sender = u64::from_be_bytes(data[8..16].try_into().unwrap());
        let receiver = u64::from_be_bytes(data[16..24].try_into().unwrap());
        let timestamp = u64::from_be_bytes(data[24..32].try_into().unwrap());
        let payload = Payload::from_bytes(&data[32..])?;
        Ok(TernaryMessage { id, sender, receiver, payload, timestamp })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trit::Trit;

    #[test]
    fn message_roundtrip() {
        let msg = TernaryMessage::new(
            MessageId(42),
            1, 2,
            Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]),
            1000,
        );
        let bytes = msg.to_bytes();
        let decoded = TernaryMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn broadcast_message() {
        let msg = TernaryMessage::broadcast(
            MessageId(1), 5,
            Payload::from_trits(&[Trit::Pos]),
            999,
        );
        assert!(msg.is_broadcast());
        assert_eq!(msg.receiver, 0);
    }

    #[test]
    fn message_too_short() {
        assert!(TernaryMessage::from_bytes(&[0u8; 16]).is_err());
    }
}
