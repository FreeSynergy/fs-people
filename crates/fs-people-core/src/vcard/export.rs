//! [`Card`] to vCard 4.0 text through `calcard`.
//!
//! Pattern: Adapter — the typed card is handed to the foreign JSContact -> vCard converter as
//! JSON; RFC 9554 parameters (`PROP-ID`, `JSCOMPS`, `JSPROP`) carry the lossless return.

use calcard::jscontact::JSContact;
use serde_json::Value;

use crate::card::Card;
use crate::error::{Error, Result};
use crate::validate::validate;

/// Export a card as vCard 4.0 text (RFC 6350 with the RFC 9554 parameters for lossless return).
pub fn card_to_vcard(card: &Card) -> Result<String> {
    validate(card)?;
    // Nested `@type` members carry no vCard information; left in, they would become JSPROP noise.
    let mut value = card.to_value();
    if let Some(o) = value.as_object_mut() {
        o.values_mut().for_each(strip_nested_types);
    }
    let json = serde_json::to_string(&value)?;
    let js = JSContact::<String, String>::parse(&json)
        .map_err(|e| Error::VCard(format!("JSContact rejected: {e}")))?;
    let v = js
        .into_vcard()
        .ok_or_else(|| Error::VCard("card cannot be expressed as vCard".into()))?;
    Ok(v.to_string())
}

fn strip_nested_types(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.remove("@type");
            o.values_mut().for_each(strip_nested_types);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_nested_types),
        _ => {}
    }
}
