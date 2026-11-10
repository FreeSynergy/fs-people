//! The `Edit` of `types.capnp` on the wire.
//!
//! Pattern: Adapter — the generated reader into the plain `Edit` of `fs-types`. The sizes are bounded
//! here, before anything is checked: an edit is typed input of a form, not a bulk channel.

use fs_interfaces::rpc::{check_intake, invalid_request};
use fs_interfaces::types_capnp::{BasicType, edit};
use fs_types::{Edit, FieldEdit};

/// At most this many fields in one edit.
const MAX_FIELDS: u32 = 64;
/// At most this many values for one field, and steps in one path.
const MAX_VALUES: u32 = 1000;
const MAX_STEPS: u32 = 8;

/// Read an edit; refuses one that is larger than a form can make.
pub(crate) fn read_edit(r: edit::Reader<'_>) -> capnp::Result<Edit> {
    let fields = r.get_fields()?;
    if fields.len() > MAX_FIELDS {
        return Err(invalid_request(format!(
            "an edit has at most {MAX_FIELDS} fields"
        )));
    }
    let mut out = Vec::new();
    for f in fields {
        let path = f.get_path()?;
        let values = f.get_values()?;
        if path.len() > MAX_STEPS || values.len() > MAX_VALUES {
            return Err(invalid_request("edit.field-too-large"));
        }
        let mut one = FieldEdit {
            path: Vec::new(),
            basic: f.get_basic().unwrap_or(BasicType::Unknown),
            values: Vec::new(),
        };
        for step in path {
            let text = step?.to_str()?;
            check_intake(text.len())?;
            one.path.push(text.to_owned());
        }
        for value in values {
            let text = value?.to_str()?;
            check_intake(text.len())?;
            one.values.push(text.to_owned());
        }
        out.push(one);
    }
    Ok(Edit { fields: out })
}
