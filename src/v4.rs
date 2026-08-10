use std::fmt::Display;

// UUIDv4 Field and Bit Layout
// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           random_a                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          random_a             |  ver  |       random_b        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |var|                       random_c                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           random_c                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UuidV4 {
    random_a: [u8; 6],
    ver_with_random_b: u16,
    var_with_random_c: u64,
}

const _: () = assert!(size_of::<UuidV4>() == 16);

const RANDOM_B_MASK: u16 = 0x0FFF;
const RANDOM_C_MASK: u64 = 0x3FFF_FFFF_FFFF_FFFF;
const VERSION_4: u16 = 0x4000;
const VARIANT_RFC4122: u64 = 0x8000_0000_0000_0000;

impl UuidV4 {
    pub fn new() -> Self {
        let mut buf = [0u8; 16];
        getrandom::fill(&mut buf).unwrap();

        Self::replace_and_construct(buf)
    }

    fn replace_and_construct(random_val: [u8; 16]) -> Self {
        let random_a: [u8; 6] = random_val[..6].try_into().unwrap();
        let random_b = u16::from_be_bytes(random_val[6..8].try_into().unwrap());
        let random_c = u64::from_be_bytes(random_val[8..].try_into().unwrap());

        let ver_with_random_b = (random_b & RANDOM_B_MASK) | VERSION_4;
        let var_with_random_c = (random_c & RANDOM_C_MASK) | VARIANT_RFC4122;

        Self {
            random_a,
            ver_with_random_b,
            var_with_random_c,
        }
    }

    fn output(&self) -> String {
        let rand_a = self
            .random_a
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let (a, b) = rand_a.split_at(8);

        let rand_c = format!("{:016x}", self.var_with_random_c);
        let (c, d) = rand_c.split_at(4);
        format!("{}-{}-{:04x}-{}-{}", a, b, self.ver_with_random_b, c, d)
    }
}

impl Display for UuidV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output())
    }
}

impl Default for UuidV4 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::v4::UuidV4;

    // Random hex: 919108f752d133205bacf847db4148a8
    // -------------------------------------------
    // field     bits value
    // -------------------------------------------
    // random_a  48   0x919108f752d1
    // ver        4   0x4
    // random_b  12   0x320
    // var        2   0b10
    // random_c  62   0b01, 0xbacf847db4148a8
    // -------------------------------------------
    // total     128
    // -------------------------------------------
    // final: 919108f7-52d1-4320-9bac-f847db4148a8
    #[test]
    fn test_v4() {
        let random_val = [
            0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x33, 0x20, 0x5b, 0xac, 0xf8, 0x47, 0xdb, 0x41,
            0x48, 0xa8,
        ];

        let val = UuidV4::replace_and_construct(random_val);
        assert_eq!(val.to_string(), "919108f7-52d1-4320-9bac-f847db4148a8");
    }
}
