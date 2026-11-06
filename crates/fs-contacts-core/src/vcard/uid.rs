//! Deterministic `uid` for vCards that have none.
//!
//! Pattern: none (pure function) — hashing the text makes re-importing the same file find the
//! same card instead of duplicating it.

/// Deterministic `urn:uuid:` (version 8, custom) from the vCard text: two FNV-1a 64 passes.
pub(super) fn uid_from_text(text: &str) -> String {
    fn fnv(seed: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(seed, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
    let t = text.trim().as_bytes();
    let (a, b) = (fnv(0xcbf2_9ce4_8422_2325, t), fnv(0x8422_2325_cbf2_9ce4, t));
    let mut x = (u128::from(a) << 64) | u128::from(b);
    x = (x & !(0xf_u128 << 76)) | (8_u128 << 76); // version 8
    x = (x & !(0x3_u128 << 62)) | (2_u128 << 62); // variant 10
    format!("urn:uuid:{}", uuid_fmt(x))
}

fn uuid_fmt(x: u128) -> String {
    let h = format!("{x:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}
