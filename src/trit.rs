/// A single ternary digit (trit): Negative (-1), Zero (0), or Positive (+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum Trit {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Trit {
    /// Create a trit from an i8 value. Returns None if value is not -1, 0, or 1.
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::Neg),
            0 => Some(Trit::Zero),
            1 => Some(Trit::Pos),
            _ => None,
        }
    }

    /// Convert to i8.
    pub fn to_i8(self) -> i8 {
        self as i8
    }

    /// Encode two trits into one byte (packed 4+4 bits).
    /// Upper nibble = first trit + 1, lower nibble = second trit + 1.
    pub fn pack_pair(a: Trit, b: Trit) -> u8 {
        ((a.to_i8() + 1) as u8) << 4 | ((b.to_i8() + 1) as u8)
    }

    /// Decode two trits from one packed byte.
    pub fn unpack_pair(byte: u8) -> (Trit, Trit) {
        let a = Trit::from_i8(((byte >> 4) & 0x0F) as i8 - 1).unwrap_or(Trit::Zero);
        let b = Trit::from_i8((byte & 0x0F) as i8 - 1).unwrap_or(Trit::Zero);
        (a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trit_from_i8_valid() {
        assert_eq!(Trit::from_i8(-1), Some(Trit::Neg));
        assert_eq!(Trit::from_i8(0), Some(Trit::Zero));
        assert_eq!(Trit::from_i8(1), Some(Trit::Pos));
    }

    #[test]
    fn trit_from_i8_invalid() {
        assert_eq!(Trit::from_i8(2), None);
        assert_eq!(Trit::from_i8(-2), None);
        assert_eq!(Trit::from_i8(127), None);
    }

    #[test]
    fn trit_roundtrip_i8() {
        for v in [-1i8, 0, 1] {
            assert_eq!(Trit::from_i8(v).unwrap().to_i8(), v);
        }
    }

    #[test]
    fn trit_pack_unpack_roundtrip() {
        let trits = [Trit::Neg, Trit::Zero, Trit::Pos];
        for &a in &trits {
            for &b in &trits {
                let packed = Trit::pack_pair(a, b);
                let (ra, rb) = Trit::unpack_pair(packed);
                assert_eq!((ra, rb), (a, b), "roundtrip failed for ({:?}, {:?})", a, b);
            }
        }
    }
}
