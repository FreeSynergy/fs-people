//! The facts behind a problem, as typed extensions.
//!
//! Pattern: none (a function that only computes from its argument). What a client used to cut out of
//! the detail text (`coding.md` §2a) is stated here as `id`, `kind`, `issue` and a few keys of the
//! people problem types; the detail text stays as it was. The table is in `DESIGN.md`.

use fs_interfaces::rpc::{Problem, problem_keys as keys};
use fs_objects::Error as StoreError;
use fs_people_core::Error;

/// The key of the number that cannot be kept exactly (`invalid`).
pub(crate) const VALUE: &str = "value";
/// The key of the `uid: name` of a sensitive card that was not released (`sensitive`).
pub(crate) const WHO: &str = "who";
/// The key of a member a list export held back (`sensitive-members`), one entry each.
pub(crate) const MEMBER: &str = "member";

/// `problem` with every fact the core error `e` knows.
pub(crate) fn with_facts(mut problem: Problem, e: &Error) -> Problem {
    if let Some(subject) = e.subject() {
        problem = problem
            .with_ext(keys::ID, &subject.id)
            .with_ext(keys::KIND, subject.kind.noun());
    } else if let Error::Store(StoreError::NotFound(id)) = e {
        problem = problem.with_ext(keys::ID, &id.to_string());
    }
    let reason = match e {
        Error::Json(inner) | Error::Store(StoreError::Serde(inner)) => Some(inner.to_string()),
        Error::Store(StoreError::Invalid(reason))
        | Error::Stored(reason)
        | Error::VCard(reason)
        | Error::PartlyDeleted(_, reason) => Some(reason.clone()),
        _ => None,
    };
    if let Some(reason) = reason {
        problem = problem.with_ext(keys::ISSUE, &reason);
    }
    match e {
        Error::Invalid(issues) => issues.iter().fold(problem, |p, issue| {
            p.with_ext(keys::ISSUE, &issue.to_string())
        }),
        Error::InexactNumber(number) => problem.with_ext(VALUE, number),
        Error::NotReleased(_, who) => problem.with_ext(WHO, who),
        Error::SensitiveMembers(_, names) => names
            .iter()
            .fold(problem, |p, name| p.with_ext(MEMBER, name)),
        _ => problem,
    }
}

#[cfg(test)]
mod tests {
    use fs_people_core::{Kind, Subject};

    use super::*;
    use crate::errors::rpc_error;

    fn problem(e: Error) -> Problem {
        Problem::from_error(&rpc_error(e)).expect("a problem")
    }

    #[test]
    fn a_list_export_names_the_list_and_each_member_held_back() {
        let list = fs_objects::ObjectId::new();
        let held = vec!["a: Ada".to_string(), "b: Bo".to_string()];
        let p = problem(Error::SensitiveMembers(
            Subject::new(Kind::List, list),
            held,
        ));
        assert_eq!(p.ext("id"), Some(list.to_string().as_str()));
        assert_eq!(p.ext("kind"), Some("list"));
        assert_eq!(p.ext_all(MEMBER).collect::<Vec<_>>(), ["a: Ada", "b: Bo"]);
    }

    #[test]
    fn a_json_error_gives_its_reason_without_the_prefix() {
        let inner = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let reason = inner.to_string();
        let p = problem(Error::Json(inner));
        assert_eq!(p.ext("issue"), Some(reason.as_str()));
        assert!(p.detail.starts_with("json: "), "{}", p.detail);
    }

    #[test]
    fn a_half_done_delete_gives_the_person_and_where_it_stopped() {
        let id = fs_objects::ObjectId::new();
        let p = problem(Error::PartlyDeleted(id, "links remain".into()));
        assert_eq!(p.ext("id"), Some(id.to_string().as_str()));
        assert_eq!(p.ext("kind"), Some("person"));
        assert_eq!(p.ext("issue"), Some("links remain"));
        assert_eq!(p.detail, "links remain; delete again to finish it");
    }
}
