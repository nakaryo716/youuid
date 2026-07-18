use std::{
    cell::LazyCell,
    sync::atomic::{AtomicU16, AtomicU64, Ordering},
};

use bytes::{Buf, BufMut, BytesMut};
use chrono::Utc;

struct V1Context {
    previous_timestamp: AtomicU64,
    clock_seq: AtomicU16,
    node: u64,
}

/// UUID Version1
///
// # Layout
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           time_low                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |           time_mid            |  ver  |       time_high       |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |var|         clock_seq         |             node              |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                              node                             |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
pub struct UuidV1 {
    time_low: u32,
    time_mid: u16,
    version_with_time_high: u16,
    variant_with_clock_seq: u16,
    node: [u8; 6],
}
const _: () = assert!(size_of::<UuidV1>() == 16);

thread_local! {
    static CTX: LazyCell<V1Context> = LazyCell::new(|| {
        let mut time_stamp = [0u8; 2];
        let mut node = [0u8; 8];

        getrandom::fill(&mut time_stamp).unwrap();
        getrandom::fill(&mut node).unwrap();

        V1Context {
            previous_timestamp: AtomicU64::new(0),
            clock_seq: AtomicU16::new(u16::from_be_bytes(time_stamp)),
            node: u64::from_be_bytes(node),
        }
    });
}

impl UuidV1 {
    pub fn generate() -> Self {
        // Unix time 1970-01-01 00:00:00
        // Gregorian 1582-10-15 00:00:00
        // diff 12_219_292_800 sec
        let now = Utc::now();
        let time_count = (now.timestamp() + 12_219_292_800) as u64 * 10_000_000
            + (now.timestamp_subsec_nanos() / 100) as u64;

        CTX.with(|cx| Self::new(time_count, cx))
    }

    fn new(time_count: u64, cx: &V1Context) -> UuidV1 {
        let mut buf = BytesMut::new();
        buf.put_u64(time_count);

        // split timestamp with time_low, time_mid and time_high
        // 0000-0000_00000000_|00000000_00000000_|00000000_00000000_00000000_00000000
        let mut time_low = buf.split_off(4);
        let mut time_mid = buf.split_off(2);

        // concat version with time_high
        let time_high_off = buf.get_u16();
        // version(0b0001) + time_high
        let version_with_time_high = time_high_off.wrapping_add(0x1000);

        if time_count <= cx.previous_timestamp.load(Ordering::Acquire) {
            cx.clock_seq.fetch_add(1, Ordering::Relaxed);
        }

        // get clock_seq
        let clock_seq = cx.clock_seq.load(Ordering::Relaxed);

        // concat variant with clock_seq
        // 0000_0000_0000_0000
        let variant_with_clock_seq = (clock_seq & 0x3FFF) | 0x8000;

        // update timestamp
        cx.previous_timestamp.store(time_count, Ordering::Release);

        // node to bytes
        //node: 0x9F 6B DE CE D8 46 u64,
        let mut node: [u8; 8] = cx.node.to_le_bytes();

        // After generating the 48-bit fully randomized node value,
        //implementations MUST set the least significant bit of the
        // first octet of the Node ID to 1 (RFC9562 6.10.)
        node[5] |= 0x01;
        let node = [node[5], node[4], node[3], node[2], node[1], node[0]];

        UuidV1 {
            time_low: time_low.get_u32(),
            time_mid: time_mid.get_u16(),
            version_with_time_high,
            variant_with_clock_seq,
            node,
        }
    }

    pub fn output(&self) -> String {
        // 4hexOctet -> 32 bits -> 32 / 4 = 8 hex digit
        let a = format!("{:08x}", self.time_low).to_uppercase();
        // 2hexOctet -> 16 bits -> 16 / 4 = 4 hex digit
        let b = format!("{:04x}", self.time_mid).to_uppercase();
        // 2hexOctet -> 16 bits -> 16 / 4 = 4 hex digit
        let c = format!("{:04x}", self.version_with_time_high).to_uppercase();
        // 2hexOctet -> 16 bits -> 16 / 4 = 4 hex digit
        let d = format!("{:04x}", self.variant_with_clock_seq).to_uppercase();

        let mut e = String::new();
        for elm in self.node {
            // each elm
            // 1hexOctet -> 8bit -> 8 / 4 = 2 hex digit
            e.push_str(&format!("{:02x}", elm).to_uppercase());
        }

        format!("{}-{}-{}-{}-{}", a, b, c, d, e)
    }

    pub fn version(&self) -> u8 {
        (self.version_with_time_high >> 12) as u8
    }

    /*
        0001_0101_0010_0010 (A)

        & 0000_1111_1111_1111  ->  (1 << 12) - 1
        -> 0000_0101_0010_0010 (WANT)

        0001_0000_0000_0000 (1 << 12)
      - 0000_0000_0000_0001

        0001_0000_0000_0000
      + 1111_1111_1111_1111
      ->0000_1111_1111_1111 (B)


        0001_0101_0010_0010 (A)
        0000_1111_1111_1111 (B)

       &0000_0101_0010_0010 (ANS)
    */

    #[allow(dead_code)]
    fn time_high(&self) -> u16 {
        (self.version_with_time_high) & ((1 << 12) - 1)
    }

    #[allow(dead_code)]
    fn variant(&self) -> u8 {
        (self.variant_with_clock_seq >> 14) as u8
    }

    #[allow(dead_code)]
    fn clock_seq(&self) -> u16 {
        (self.variant_with_clock_seq) & ((1 << 14) - 1)
    }
}

#[cfg(test)]
mod tests {
    use crate::v1::*;

    #[test]
    fn version() {
        let uuid = UuidV1 {
            time_low: 0,
            time_mid: 0,
            version_with_time_high: 0b0001_0110_1001_1101,
            variant_with_clock_seq: 0,
            node: [0; 6],
        };

        assert_eq!(uuid.version(), 1);
    }

    #[test]
    fn time_high() {
        let uuid = UuidV1 {
            time_low: 0,
            time_mid: 0,
            version_with_time_high: 0b0001_0110_1001_1101,
            variant_with_clock_seq: 0,
            node: [0; 6],
        };

        assert_eq!(uuid.time_high(), 0b0000_0110_1001_1101);
    }

    #[test]
    fn variant() {
        let uuid = UuidV1 {
            time_low: 0,
            time_mid: 0,
            version_with_time_high: 0,
            variant_with_clock_seq: 0b1011_0110_1001_1101,
            node: [0; 6],
        };

        assert_eq!(uuid.variant(), 2);
    }

    #[test]
    fn clock_seq() {
        let uuid = UuidV1 {
            time_low: 0,
            time_mid: 0,
            version_with_time_high: 0,
            variant_with_clock_seq: 0b1011_0110_1001_1101,
            node: [0; 6],
        };

        assert_eq!(uuid.clock_seq(), 0b0011_0110_1001_1101);
    }

    // # Test case
    //   param     bits   value
    // - time_low   32   0xC232AB00
    // - time_mid   16   0x9414
    // - ver         4   0x1
    // - time_high  12   0x1EC
    // - var         2   0b10
    // - clock_seq  14   0b11, 0x3C8
    // - node       48   0x9F6BDECED846
    //
    // final: C232AB00-9414-11EC-B3C8-9F6BDECED846
    #[test]
    fn new() {
        let timecount = 0x1EC_9414_C232AB00;
        let cx = V1Context {
            previous_timestamp: AtomicU64::new(0),
            clock_seq: AtomicU16::new(0x33C8),
            node: 0x9F_6B_DE_CE_D8_46u64,
        };

        let uuid = UuidV1::new(timecount, &cx);
        assert_eq!(uuid.output(), "C232AB00-9414-11EC-B3C8-9F6BDECED846")
    }

    #[test]
    fn test_node_id_auto_correction() {
        let timecount = 0x1EC_9414_C232AB00;
        let cx = V1Context {
            previous_timestamp: AtomicU64::new(0),
            clock_seq: AtomicU16::new(0x33C8),
            node: 0x9E_6B_DE_CE_D8_46u64,
        };

        let uuid = UuidV1::new(timecount, &cx);

        assert_eq!(uuid.output(), "C232AB00-9414-11EC-B3C8-9F6BDECED846");
    }

    #[test]
    fn generate() {
        let a = UuidV1::generate();
        assert_eq!(a.version(), 1);
    }
}
