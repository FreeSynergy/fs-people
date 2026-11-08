//! Where a pair of cards stands: proposed, confirmed or rejected.
//!
//! Pattern: State (small, a table) — a pair a person has not answered is `Proposed`; they link it
//! (`Confirmed`) or say no (`Rejected`), and a rejection can be taken back. The table is the one
//! place that says which step is allowed from which state, so confirm, reject and unlink cannot
//! drift apart. A pair that was rejected is never proposed again (ADR 0058), but a person can link
//! it by hand at any time.

use crate::error::{Error, Result};

/// The answer so far for a pair of cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalState {
    /// Found by the rules, not answered.
    Proposed,
    /// Linked as the same person.
    Confirmed,
    /// Declined; the pair is not proposed again.
    Rejected,
}

/// What a person does with a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Link the pair.
    Confirm,
    /// Say it is not the same person.
    Reject,
    /// Take a rejection back: the pair may be proposed again.
    Reopen,
    /// Take a link back; the pair counts as rejected, so the system does not ask again.
    Unlink,
}

impl ProposalState {
    /// The state after `step`.
    ///
    /// # Errors
    /// The step makes no sense from this state (rejecting a linked pair, reopening one that was not rejected).
    pub fn on(self, step: Step) -> Result<Self> {
        use ProposalState::{Confirmed, Proposed, Rejected};
        Ok(match (self, step) {
            // Linking by hand a pair that was rejected is the person's own call.
            (Proposed | Rejected, Step::Confirm) => Confirmed,
            (Proposed, Step::Reject) | (Confirmed, Step::Unlink) => Rejected,
            (Rejected, Step::Reopen) => Proposed,
            (state, step) => {
                return Err(Error::Conflict(format!(
                    "a pair that is {state:?} cannot take the step {step:?}"
                )));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ProposalState::{Confirmed, Proposed, Rejected};
    use super::Step::{Confirm, Reject, Reopen, Unlink};

    #[test]
    fn the_table() {
        assert_eq!(Proposed.on(Confirm).unwrap(), Confirmed);
        assert_eq!(Proposed.on(Reject).unwrap(), Rejected);
        assert_eq!(Rejected.on(Reopen).unwrap(), Proposed);
        assert_eq!(Rejected.on(Confirm).unwrap(), Confirmed);
        assert_eq!(Confirmed.on(Unlink).unwrap(), Rejected);
        for (state, step) in [
            (Confirmed, Confirm),
            (Confirmed, Reject),
            (Confirmed, Reopen),
            (Proposed, Reopen),
            (Proposed, Unlink),
            (Rejected, Reject),
            (Rejected, Unlink),
        ] {
            assert!(state.on(step).is_err(), "{state:?} {step:?}");
        }
    }
}
