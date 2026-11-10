//! `check`: the card against the rules of RFC 9553 and of the basic types.
//!
//! Pattern: Template method — one flow: the structural rules of the card (`validate`), then each
//! field of the description through the rule of its basic type (`fs-types`). It reports; it never
//! changes the card. A problem is a path and a code, never a sentence (ADR 0057).

use std::collections::BTreeMap;

use fs_types::{BasicRules, DescribesType, RecordProblem};

use super::fields;
use crate::card::Card;
use crate::error::Issue;
use crate::validate::issues;

/// All problems of `card`: RFC 9553 first, then the values by basic type. Empty = in order.
#[must_use]
pub fn check(card: &Card) -> Vec<Issue> {
    let mut found = issues(card);
    let description = Card::describe();
    let rules = BasicRules::standard();
    let table = fields::table();
    for f in table.iter().filter(|f| f.checked) {
        let read = (f.read)(card);
        let record: BTreeMap<String, Vec<String>> = [(
            f.name.to_owned(),
            read.iter().map(|(_, v)| v.clone()).collect(),
        )]
        .into();
        for issue in description.check_record(&rules, &record) {
            let id = read.get(issue.index).map_or("", |(id, _)| id.as_str());
            let at = if id.is_empty() {
                format!("/{}", f.name)
            } else {
                format!("/{}/{id}", f.name)
            };
            found.push(Issue {
                path: at,
                message: code_of(issue.problem).to_owned(),
            });
        }
    }
    found
}

/// The translation key suffix of a finding (`problem.<code>`).
pub(super) fn code_of(problem: RecordProblem) -> &'static str {
    match problem {
        RecordProblem::Value(p) => p.code(),
        RecordProblem::UnknownField => "unknown-field",
        RecordProblem::Missing => "missing",
        RecordProblem::NotMultiple => "not-multiple",
        RecordProblem::WrongType => "wrong-type",
        RecordProblem::NestedPath => "nested-path",
        RecordProblem::DuplicatePath => "duplicate-path",
    }
}
