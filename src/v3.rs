use std::fmt::Display;

use bytes::Buf;
use md5::{Digest, Md5};

// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                            md5_high                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          md5_high             |  ver  |       md5_mid         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |var|                        md5_low                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                            md5_low                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UuidV3 {
    md5_high: [u8; 6],
    ver_with_md5_mid: u16,
    var_with_md5_low: u64,
}

const _: () = assert!(size_of::<UuidV3>() == 16);

impl UuidV3 {
    pub fn new(namespace_id: &[u8; 16], name: &str) -> Self {
        let mut hasher = Md5::new();
        hasher.update(namespace_id);
        hasher.update(name);
        let hash = hasher.finalize();

        let md5_high = hash[..6].try_into().unwrap();
        let md5_mid = u16::from_be_bytes(hash[6..8].try_into().unwrap());
        let md5_low = u64::from_be_bytes(hash[8..].try_into().unwrap());

        let ver_with_md5_mid = (md5_mid & 0x0FFF) | 0x3000;
        let var_with_md5_low = (md5_low & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;

        Self {
            md5_high,
            ver_with_md5_mid,
            var_with_md5_low,
        }
    }

    pub fn ver(&self) -> u8 {
        (self.ver_with_md5_mid >> 12) as u8
    }

    pub fn md5_mid(&self) -> u16 {
        self.ver_with_md5_mid & ((1 << 12) - 1)
    }

    pub fn var(&self) -> u8 {
        (self.var_with_md5_low >> 62) as u8
    }

    pub fn md5_low(&self) -> u64 {
        self.var_with_md5_low & ((1 << 62) - 1)
    }

    // UUID     = 4hexOctet "-"
    //            2hexOctet "-"
    //            2hexOctet "-"
    //            2hexOctet "-"
    //            6hexOctet
    fn output(&self) -> String {
        let a = self.md5_high[..4].iter().as_slice().try_get_u32().unwrap();
        let b = self.md5_high[4..].iter().as_slice().try_get_u16().unwrap();
        let c = self.ver_with_md5_mid;

        let byte = self.var_with_md5_low.to_be_bytes();
        let remain_bytes: [u8; 6] = byte[2..].try_into().unwrap();

        let d = byte[..2].iter().as_slice().try_get_u16().unwrap();

        let mut e = String::new();
        for i in remain_bytes {
            e.push_str(format!("{:02x}", i).as_str());
        }

        format!("{:08x}-{:04x}-{:04x}-{:04x}-{}", a, b, c, d, e)
    }
}

impl Display for UuidV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::v3::UuidV3;

    // Namespace (DNS):  6ba7b810-9dad-11d1-80b4-00c04fd430c8
    // Name:             www.example.com
    // ------------------------------------------------------
    // MD5:              5df418813aed051548a72f4a814cf09e
    // Figure 17: UUIDv3 Example MD5
    // -------------------------------------------
    // field     bits value
    // -------------------------------------------
    // md5_high  48   0x5df418813aed
    // ver        4   0x3
    // md5_mid   12   0x515
    // var        2   0b10
    // md5_low   62   0b00, 0x8a72f4a814cf09e
    // -------------------------------------------
    // total     128
    // -------------------------------------------
    // final: 5df41881-3aed-3515-88a7-2f4a814cf09e
    #[test]
    fn test() {
        let val: [u8; 16] = [
            0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4,
            0x30, 0xc8,
        ];

        let v3 = UuidV3::new(&val, "www.example.com");

        let mut md5_high: VecDeque<_> = v3.md5_high.to_vec().into();
        md5_high.push_front(0);
        md5_high.push_front(0);
        let va: Vec<u8> = md5_high.into();

        let val = va.try_into().unwrap();
        // md5_high
        assert_eq!(u64::from_be_bytes(val), 0x5df418813aed);

        // version
        assert_eq!(v3.ver(), 0x3);

        // md5_mid
        assert_eq!(v3.md5_mid(), 0x515);

        // var
        assert_eq!(v3.var(), 0b10);

        // md5_low
        assert_eq!(v3.md5_low(), 0x8a72f4a814cf09e);

        // final output
        assert_eq!(v3.to_string(), "5df41881-3aed-3515-88a7-2f4a814cf09e");
    }
}
