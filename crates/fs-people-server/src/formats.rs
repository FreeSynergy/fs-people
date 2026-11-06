//! Which media types this server reads and writes.
//!
//! Pattern: Registry — media type to format of the core. A type nobody writes is the explicit "not
//! possible" of the provider rule (ADR 0045), not a silent fallback to another format.

use fs_interfaces::Unsupported;
use fs_interfaces::rpc::unsupported;
use fs_people_core::{Exporter, Importer, JsContactFormat, VCardFormat};

const IMPORTERS: [&dyn Importer; 2] = [&VCardFormat, &JsContactFormat];
const EXPORTERS: [&dyn Exporter; 2] = [&VCardFormat, &JsContactFormat];

/// The essence of a media type: without parameters, in lower case.
fn essence(media_type: &str) -> String {
    let essence = media_type.split(';').next().unwrap_or_default();
    essence.trim().to_ascii_lowercase()
}

fn no_format(capability: &str, media_type: &str) -> capnp::Error {
    unsupported(&Unsupported::new(
        capability,
        format!(
            "no format for the media type {media_type:?}; text/vcard and application/jscontact+json exist"
        ),
    ))
}

/// The importer of a media type.
pub(crate) fn importer(media_type: &str) -> capnp::Result<&'static dyn Importer> {
    let wanted = essence(media_type);
    IMPORTERS
        .into_iter()
        .find(|f| f.media_type() == wanted)
        .ok_or_else(|| no_format("person.import", media_type))
}

/// The exporter of a media type.
pub(crate) fn exporter(media_type: &str) -> capnp::Result<&'static dyn Exporter> {
    let wanted = essence(media_type);
    EXPORTERS
        .into_iter()
        .find(|f| f.output_media_type() == wanted)
        .ok_or_else(|| no_format("person.export", media_type))
}
