use crate::trit::Trit;

/// A ternary-encoded payload: a sequence of trits with compact binary serialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Payload {
    trits: Vec<Trit>,
}

impl Payload {
    /// Create an empty payload.
    pub fn new() -> Self {
        Payload { trits: Vec::new() }
    }

    /// Create a payload from a slice of trits.
    pub fn from_trits(trits: &[Trit]) -> Self {
        Payload { trits: trits.to_vec() }
    }

    /// Number of trits in the payload.
    pub fn len(&self) -> usize {
        self.trits.len()
    }

    /// Is the payload empty?
    pub fn is_empty(&self) -> bool {
        self.trits.is_empty()
    }

    /// Access the trits.
    pub fn trits(&self) -> &[Trit] {
        &self.trits
    }

    /// Push a trit onto the payload.
    pub fn push(&mut self, t: Trit) {
        self.trits.push(t);
    }

    /// Append another payload.
    pub fn extend(&mut self, other: &Payload) {
        self.trits.extend_from_slice(&other.trits);
    }

    /// Encode payload to compact binary format.
    ///
    /// Format: [4 bytes big-endian length] [packed trit pairs]
    /// Each byte holds two trits. If odd number, last byte has padding trit = Zero.
    pub fn to_bytes(&self) -> Vec<u8> {
        let n = self.trits.len();
        let mut out = Vec::with_capacity(4 + (n + 1) / 2);
        // Length as u32 big-endian
        out.extend_from_slice(&(n as u32).to_be_bytes());
        // Pack pairs of trits
        let mut i = 0;
        while i + 1 < n {
            out.push(Trit::pack_pair(self.trits[i], self.trits[i + 1]));
            i += 2;
        }
        if i < n {
            out.push(Trit::pack_pair(self.trits[i], Trit::Zero));
        }
        out
    }

    /// Decode payload from compact binary format.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("payload too short: need at least 4 bytes for length".into());
        }
        let n = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let body = &data[4..];
        let bytes_needed = (n + 1) / 2;
        if body.len() < bytes_needed {
            return Err(format!(
                "payload truncated: need {} bytes for {} trits, got {}",
                bytes_needed,
                n,
                body.len()
            ));
        }
        let mut trits = Vec::with_capacity(n);
        let mut i = 0;
        while trits.len() < n {
            if i >= body.len() {
                return Err("ran out of bytes while decoding trits".into());
            }
            let (a, b) = Trit::unpack_pair(body[i]);
            trits.push(a);
            if trits.len() < n {
                trits.push(b);
            }
            i += 1;
        }
        Ok(Payload { trits })
    }

    /// XOR-like ternary addition (balanced ternary sum clamped to -1..1).
    /// Both payloads must be the same length.
    pub fn ternary_xor(&self, other: &Payload) -> Result<Payload, String> {
        if self.len() != other.len() {
            return Err(format!(
                "length mismatch: {} vs {}",
                self.len(),
                other.len()
            ));
        }
        let trits: Vec<Trit> = self.trits.iter().zip(other.trits.iter())
            .map(|(&a, &b)| {
                let sum = a.to_i8() + b.to_i8();
                Trit::from_i8(sum.clamp(-1, 1)).unwrap()
            })
            .collect();
        Ok(Payload { trits })
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_empty() {
        let p = Payload::new();
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn payload_from_trits() {
        let p = Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]);
        assert_eq!(p.len(), 3);
        assert_eq!(p.trits(), &[Trit::Pos, Trit::Neg, Trit::Zero]);
    }

    #[test]
    fn payload_serialize_roundtrip() {
        let original = Payload::from_trits(&[
            Trit::Pos, Trit::Neg, Trit::Zero, Trit::Pos,
            Trit::Neg, Trit::Zero, Trit::Pos, Trit::Pos,
            Trit::Neg,
        ]);
        let bytes = original.to_bytes();
        let decoded = Payload::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn payload_empty_roundtrip() {
        let original = Payload::new();
        let bytes = original.to_bytes();
        let decoded = Payload::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn payload_too_short() {
        assert!(Payload::from_bytes(&[0, 0]).is_err());
    }
}
