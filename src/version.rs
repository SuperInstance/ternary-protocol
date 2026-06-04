/// Protocol version for forward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Errors during version negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionError {
    Incompatible { ours: ProtocolVersion, theirs: ProtocolVersion },
}

impl ProtocolVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        ProtocolVersion { major, minor, patch }
    }

    /// Current protocol version.
    pub fn current() -> Self {
        ProtocolVersion::new(0, 1, 0)
    }

    /// Check if this version is compatible with another.
    /// Same major version = compatible.
    pub fn is_compatible_with(&self, other: &ProtocolVersion) -> bool {
        self.major == other.major
    }

    /// Encode to 12 bytes (three u32 big-endian).
    pub fn to_bytes(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..4].copy_from_slice(&self.major.to_be_bytes());
        out[4..8].copy_from_slice(&self.minor.to_be_bytes());
        out[8..12].copy_from_slice(&self.patch.to_be_bytes());
        out
    }

    /// Decode from 12 bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 12 {
            return Err("need 12 bytes for version".into());
        }
        let major = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let minor = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let patch = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        Ok(ProtocolVersion { major, minor, patch })
    }

    /// Negotiate the best version between ours and a list of peer versions.
    /// Returns the highest compatible version, or error if none match.
    pub fn negotiate(ours: ProtocolVersion, peers: &[ProtocolVersion]) -> Result<ProtocolVersion, VersionError> {
        let compatible: Vec<&ProtocolVersion> = peers.iter()
            .filter(|v| ours.is_compatible_with(v))
            .collect();
        if compatible.is_empty() {
            return Err(VersionError::Incompatible {
                ours,
                theirs: peers.first().copied().unwrap_or(ProtocolVersion::new(0, 0, 0)),
            });
        }
        // Return highest compatible
        Ok(*compatible.into_iter().max().unwrap())
    }

    /// Display string.
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_bytes_roundtrip() {
        let v = ProtocolVersion::new(1, 2, 3);
        let bytes = v.to_bytes();
        let decoded = ProtocolVersion::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn version_compatibility() {
        let v1 = ProtocolVersion::new(1, 0, 0);
        let v2 = ProtocolVersion::new(1, 5, 3);
        let v3 = ProtocolVersion::new(2, 0, 0);
        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn version_negotiate_success() {
        let ours = ProtocolVersion::new(1, 2, 0);
        let peers = vec![
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(1, 5, 0),
            ProtocolVersion::new(1, 3, 0),
        ];
        let best = ProtocolVersion::negotiate(ours, &peers).unwrap();
        assert_eq!(best, ProtocolVersion::new(1, 5, 0));
    }

    #[test]
    fn version_negotiate_fail() {
        let ours = ProtocolVersion::new(3, 0, 0);
        let peers = vec![ProtocolVersion::new(1, 0, 0), ProtocolVersion::new(2, 0, 0)];
        let result = ProtocolVersion::negotiate(ours, &peers);
        assert!(matches!(result, Err(VersionError::Incompatible { .. })));
    }

    #[test]
    fn version_display() {
        let v = ProtocolVersion::new(0, 1, 0);
        assert_eq!(v.to_string(), "0.1.0");
    }
}
