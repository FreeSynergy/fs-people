//! vCard 4.0 (RFC 6350) import/export through `calcard` 0.3.14 (MIT OR Apache-2.0), which maps
//! vCard <-> JSContact following RFC 9554. The imported text is kept byte for byte as the
//! original (ADR 0030).

use calcard::jscontact::JSContact;
use calcard::vcard::VCard;
use fs_store::Blob;

use crate::card::Card;
use crate::error::{Error, Result};
use crate::validate::validate;

pub const VCARD_MEDIA_TYPE: &str = "text/vcard";

/// A card parsed from a vCard together with its original text.
#[derive(Clone, Debug)]
pub struct ImportedCard {
    pub card: Card,
    pub original: Blob,
}

/// Split a `.vcf` stream into the exact text of each `BEGIN:VCARD` .. `END:VCARD` block.
pub fn split_vcards(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.eq_ignore_ascii_case("BEGIN:VCARD") {
            start = Some(pos);
        } else if t.eq_ignore_ascii_case("END:VCARD") {
            if let Some(s) = start.take() {
                out.push(&text[s..pos + line.len()]);
            }
        }
        pos += line.len();
    }
    out
}

/// Convert one vCard (any version calcard understands: 2.1, 3.0, 4.0) to a validated card.
pub fn card_from_vcard(vcard_text: &str) -> Result<Card> {
    let vcard = VCard::parse(vcard_text).map_err(|e| Error::VCard(format!("cannot parse vCard: {e:?}")))?;
    let js = vcard.into_jscontact::<String, String>();
    let mut card: Card = serde_json::from_str(&js.to_string_pretty())?;
    if card.uid.trim().is_empty() {
        // vCard 3.0 and earlier do not require UID; we do (stable exchange). Derived from the
        // text, so importing the same file twice finds the card again instead of duplicating it.
        card.uid = uid_from_text(vcard_text);
    }
    if card.type_.is_empty() {
        card.type_ = "Card".into();
    }
    validate(&card)?;
    Ok(card)
}

/// Import every vCard in `text`. Blocks that fail to convert are reported, not silently dropped.
pub fn import_vcards(text: &str) -> Vec<Result<ImportedCard>> {
    split_vcards(text)
        .into_iter()
        .map(|block| {
            let card = card_from_vcard(block)?;
            Ok(ImportedCard {
                card,
                original: Blob { media_type: VCARD_MEDIA_TYPE.into(), bytes: block.as_bytes().to_vec() },
            })
        })
        .collect()
}

/// Export a card as vCard 4.0 text (RFC 6350 with the RFC 9554 parameters for lossless return).
pub fn card_to_vcard(card: &Card) -> Result<String> {
    validate(card)?;
    // Nested `@type` members carry no vCard information; left in, they would become JSPROP noise.
    let mut value = card.to_value();
    if let Some(o) = value.as_object_mut() {
        o.values_mut().for_each(strip_nested_types);
    }
    let json = serde_json::to_string(&value)?;
    let js = JSContact::<String, String>::parse(&json).map_err(|e| Error::VCard(format!("JSContact rejected: {e}")))?;
    let v = js.into_vcard().ok_or_else(|| Error::VCard("card cannot be expressed as vCard".into()))?;
    Ok(v.to_string())
}

/// Deterministic `urn:uuid:` (version 8, custom) from the vCard text: two FNV-1a 64 passes.
fn uid_from_text(text: &str) -> String {
    fn fnv(seed: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(seed, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3))
    }
    let t = text.trim().as_bytes();
    let (a, b) = (fnv(0xcbf2_9ce4_8422_2325, t), fnv(0x84222325cbf29ce4, t));
    let mut x = ((u128::from(a)) << 64) | u128::from(b);
    x = (x & !(0xf_u128 << 76)) | (8_u128 << 76); // version 8
    x = (x & !(0x3_u128 << 62)) | (2_u128 << 62); // variant 10
    format!("urn:uuid:{}", uuid_fmt(x))
}

fn uuid_fmt(x: u128) -> String {
    let h = format!("{x:032x}");
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn strip_nested_types(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            o.remove("@type");
            o.values_mut().for_each(strip_nested_types);
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(strip_nested_types),
        _ => {}
    }
}
