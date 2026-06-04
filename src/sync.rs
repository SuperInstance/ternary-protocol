use crate::payload::Payload;
use crate::trit::Trit;

/// A diff operation: insert, remove, or replace trits at a given position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
    /// Insert trits at position.
    Insert { pos: usize, trits: Vec<Trit> },
    /// Remove `count` trits starting at position.
    Remove { pos: usize, count: usize },
    /// Replace trits starting at position.
    Replace { pos: usize, trits: Vec<Trit> },
}

/// A diff between two payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub ops: Vec<DiffOp>,
}

impl Diff {
    pub fn new(ops: Vec<DiffOp>) -> Self {
        Diff { ops }
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Encode diff to bytes.
    /// Format per op: [1B type] [8B pos BE] [4B len BE] [packed trits (for Insert/Replace)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // Number of ops as u32
        out.extend_from_slice(&(self.ops.len() as u32).to_be_bytes());
        for op in &self.ops {
            match op {
                DiffOp::Insert { pos, trits } => {
                    out.push(0u8);
                    out.extend_from_slice(&(*pos as u64).to_be_bytes());
                    out.extend_from_slice(&(trits.len() as u32).to_be_bytes());
                    let p = Payload::from_trits(trits);
                    out.extend_from_slice(&p.to_bytes());
                }
                DiffOp::Remove { pos, count } => {
                    out.push(1u8);
                    out.extend_from_slice(&(*pos as u64).to_be_bytes());
                    out.extend_from_slice(&(*count as u32).to_be_bytes());
                }
                DiffOp::Replace { pos, trits } => {
                    out.push(2u8);
                    out.extend_from_slice(&(*pos as u64).to_be_bytes());
                    out.extend_from_slice(&(trits.len() as u32).to_be_bytes());
                    let p = Payload::from_trits(trits);
                    out.extend_from_slice(&p.to_bytes());
                }
            }
        }
        out
    }

    /// Decode diff from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("diff too short".into());
        }
        let num_ops = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let mut ops = Vec::with_capacity(num_ops);
        let mut offset = 4;
        for _ in 0..num_ops {
            if offset >= data.len() {
                return Err("unexpected end of diff".into());
            }
            let op_type = data[offset];
            offset += 1;
            if offset + 12 > data.len() {
                return Err("truncated op header".into());
            }
            let pos = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            let len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            match op_type {
                0 => {
                    // Insert - need to read payload bytes
                    let payload_len = 4 + (len + 1) / 2;
                    if offset + payload_len > data.len() {
                        return Err("truncated insert payload".into());
                    }
                    let payload = Payload::from_bytes(&data[offset..offset + payload_len])?;
                    offset += payload_len;
                    ops.push(DiffOp::Insert { pos, trits: payload.trits().to_vec() });
                }
                1 => {
                    ops.push(DiffOp::Remove { pos, count: len });
                }
                2 => {
                    let payload_len = 4 + (len + 1) / 2;
                    if offset + payload_len > data.len() {
                        return Err("truncated replace payload".into());
                    }
                    let payload = Payload::from_bytes(&data[offset..offset + payload_len])?;
                    offset += payload_len;
                    ops.push(DiffOp::Replace { pos, trits: payload.trits().to_vec() });
                }
                _ => return Err(format!("unknown op type: {}", op_type)),
            }
        }
        Ok(Diff { ops })
    }
}

/// Errors during sync operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    InvalidPosition { pos: usize, len: usize },
}

/// Synchronization protocol: compute diffs and apply them to payloads.
pub struct SyncProtocol;

impl SyncProtocol {
    /// Compute a simple diff between two payloads.
    /// For simplicity: finds the first differing position and generates ops.
    /// This is a basic line-level diff — replace changed suffix.
    pub fn diff(old: &Payload, new: &Payload) -> Diff {
        let old_t = old.trits();
        let new_t = new.trits();
        let common = old_t.iter().zip(new_t.iter())
            .take_while(|(a, b)| a == b)
            .count();
        if common == old_t.len() && common == new_t.len() {
            return Diff::new(Vec::new());
        }
        // If one is a prefix of the other
        if common == old_t.len() {
            return Diff::new(vec![DiffOp::Insert {
                pos: common,
                trits: new_t[common..].to_vec(),
            }]);
        }
        if common == new_t.len() {
            return Diff::new(vec![DiffOp::Remove {
                pos: common,
                count: old_t.len() - common,
            }]);
        }
        // General: remove remaining old, insert remaining new
        let mut ops = Vec::new();
        let old_remaining = old_t.len() - common;
        let new_remaining = &new_t[common..];
        if old_remaining > 0 {
            ops.push(DiffOp::Remove { pos: common, count: old_remaining });
        }
        if !new_remaining.is_empty() {
            ops.push(DiffOp::Insert { pos: common, trits: new_remaining.to_vec() });
        }
        Diff::new(ops)
    }

    /// Apply a diff to a payload, producing the new payload.
    pub fn apply(base: &Payload, diff: &Diff) -> Result<Payload, SyncError> {
        let mut trits = base.trits().to_vec();
        // Apply ops in order (offsets are relative to the evolving state)
        for op in &diff.ops {
            match op {
                DiffOp::Insert { pos, trits: new_trits } => {
                    if *pos > trits.len() {
                        return Err(SyncError::InvalidPosition { pos: *pos, len: trits.len() });
                    }
                    for (i, t) in new_trits.iter().enumerate() {
                        trits.insert(pos + i, *t);
                    }
                }
                DiffOp::Remove { pos, count } => {
                    if *pos + count > trits.len() {
                        return Err(SyncError::InvalidPosition { pos: *pos + count, len: trits.len() });
                    }
                    trits.drain(*pos..*pos + count);
                }
                DiffOp::Replace { pos, trits: new_trits } => {
                    if *pos + new_trits.len() > trits.len() {
                        return Err(SyncError::InvalidPosition { pos: *pos + new_trits.len(), len: trits.len() });
                    }
                    for (i, t) in new_trits.iter().enumerate() {
                        trits[pos + i] = *t;
                    }
                }
            }
        }
        Ok(Payload::from_trits(&trits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical() {
        let p = Payload::from_trits(&[Trit::Pos, Trit::Neg]);
        let d = SyncProtocol::diff(&p, &p);
        assert!(d.is_empty());
    }

    #[test]
    fn diff_insert_suffix() {
        let old = Payload::from_trits(&[Trit::Pos]);
        let new = Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]);
        let d = SyncProtocol::diff(&old, &new);
        let result = SyncProtocol::apply(&old, &d).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn diff_remove_suffix() {
        let old = Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]);
        let new = Payload::from_trits(&[Trit::Pos]);
        let d = SyncProtocol::diff(&old, &new);
        let result = SyncProtocol::apply(&old, &d).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn diff_replace() {
        let old = Payload::from_trits(&[Trit::Pos, Trit::Pos, Trit::Pos]);
        let new = Payload::from_trits(&[Trit::Neg, Trit::Neg]);
        let d = SyncProtocol::diff(&old, &new);
        let result = SyncProtocol::apply(&old, &d).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn diff_roundtrip_bytes() {
        let diff = Diff::new(vec![
            DiffOp::Insert { pos: 0, trits: vec![Trit::Pos, Trit::Neg] },
            DiffOp::Remove { pos: 5, count: 3 },
            DiffOp::Replace { pos: 2, trits: vec![Trit::Zero] },
        ]);
        let bytes = diff.to_bytes();
        let decoded = Diff::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, diff);
    }
}
