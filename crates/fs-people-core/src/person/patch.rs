//! `PatchObject`: a partial change of a card (RFC 8984 section 1.4.9, used by RFC 9553).
//!
//! Pattern: Command — a patch is a set of independent edits (set a value, remove one), each named
//! by a path; `apply` runs them on a JSON value and changes nothing when one is refused.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{Error, Issue, Result};

/// Path to value; a `null` value removes what the path names.
///
/// A path is a JSON pointer (RFC 6901) without its first `/`: `emails/e1/address`. `~1` stands
/// for a `/` and `~0` for a `~` inside a key. It never goes through an array, every step before the
/// last must exist, and no path may lie inside another one of the same patch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchObject(BTreeMap<String, Value>);

impl PatchObject {
    /// A patch from its JSON text (an object).
    pub fn parse(text: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(text).map_err(|_| refuse("", "patch.not-json"))?;
        Self::from_value(value)
    }

    /// A patch from a JSON object.
    pub fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Object(map) => Ok(Self(map.into_iter().collect())),
            _ => Err(refuse("", "patch.not-object")),
        }
    }

    /// The edits, by path.
    #[must_use]
    pub fn edits(&self) -> &BTreeMap<String, Value> {
        &self.0
    }

    /// Run every edit on `target`. All or nothing: on an error `target` is as it was.
    pub fn apply(&self, target: &mut Value) -> Result<()> {
        self.check_paths()?;
        let mut work = target.clone();
        for (path, value) in &self.0 {
            let steps = steps(path)?;
            let (last, parents) = steps
                .split_last()
                .ok_or_else(|| refuse(path, "patch.empty-path"))?;
            let mut at = &mut work;
            for step in parents {
                at = at
                    .as_object_mut()
                    .and_then(|o| o.get_mut(step))
                    .ok_or_else(|| refuse(path, "patch.step-missing"))?;
            }
            let object = at
                .as_object_mut()
                .ok_or_else(|| refuse(path, "patch.parent-not-object"))?;
            if value.is_null() {
                object.remove(last);
            } else {
                object.insert(last.clone(), value.clone());
            }
        }
        *target = work;
        Ok(())
    }

    /// No path may be a prefix of another one (RFC 8984: they could contradict each other).
    fn check_paths(&self) -> Result<()> {
        let paths: Vec<&String> = self.0.keys().collect();
        for (i, a) in paths.iter().enumerate() {
            for b in &paths[i + 1..] {
                if b.starts_with(&format!("{a}/")) || a.starts_with(&format!("{b}/")) {
                    return Err(refuse(a, "patch.paths-overlap"));
                }
            }
        }
        Ok(())
    }
}

fn steps(path: &str) -> Result<Vec<String>> {
    path.split('/')
        .map(|s| {
            if s.is_empty() {
                return Err(refuse(path, "patch.empty-step"));
            }
            let mut out = String::new();
            let mut chars = s.chars();
            while let Some(c) = chars.next() {
                if c != '~' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('0') => out.push('~'),
                    Some('1') => out.push('/'),
                    _ => return Err(refuse(path, "patch.bad-escape")),
                }
            }
            Ok(out)
        })
        .collect()
}

fn refuse(path: &str, message: &str) -> Error {
    Error::Invalid(vec![Issue {
        path: format!("/{path}"),
        message: message.into(),
    }])
}
