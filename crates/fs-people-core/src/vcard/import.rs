//! vCard text to validated [`Card`]s through `calcard` (also reads vCard 2.1 and 3.0).
//!
//! Pattern: Adapter — `calcard`'s JSContact model is foreign; we exchange JSON text at the
//! boundary and own the typed `Card`. The imported text is kept as the original (ADR 0030).

use calcard::vcard::VCard;
use fs_store::Blob;

use super::split::scan;
use super::uid::uid_from_text;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::format::ImportedCard;
use crate::validate::{check_exact_numbers, validate};

/// Media type of the stored original.
pub const VCARD_MEDIA_TYPE: &str = "text/vcard";

/// Convert one vCard (any version calcard understands: 2.1, 3.0, 4.0) to a validated card.
pub fn card_from_vcard(vcard_text: &str) -> Result<Card> {
    let vcard =
        VCard::parse(vcard_text).map_err(|e| Error::VCard(format!("cannot parse vCard: {e:?}")))?;
    let js = vcard.into_jscontact::<String, String>();
    let json = js.to_string_pretty();
    check_exact_numbers(&json)?;
    let mut card: Card = serde_json::from_str(&json)?;
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

/// Import every vCard in `text`. Blocks that fail to convert and lines outside any complete block
/// are reported, not silently dropped.
pub fn import_vcards(text: &str) -> Vec<Result<ImportedCard>> {
    each_vcard(text).collect()
}

/// [`import_vcards`], one card at a time.
pub fn each_vcard(text: &str) -> impl Iterator<Item = Result<ImportedCard>> + Send + '_ {
    let (blocks, lost) = scan(text);
    let cards = blocks.into_iter().map(|block| {
        let card = card_from_vcard(block)?;
        let original = Blob {
            media_type: VCARD_MEDIA_TYPE.into(),
            bytes: block.as_bytes().to_vec(),
        };
        Ok(ImportedCard {
            card,
            original: Some(original),
        })
    });
    let lost = (lost > 0).then(|| {
        Err(Error::VCard(format!(
            "{lost} lines are outside any complete vCard and were not read"
        )))
    });
    cards.chain(lost)
}
