//! The answer of one job, which cannot be lost without a word.
//!
//! Pattern: RAII / Guard — it is made when the job starts running. If the job is dropped after that
//! without having answered (it panicked, or the connection thread was shut down under it), `Drop`
//! answers with an error that says so, instead of the caller seeing a silent "connection gone".

use fs_people_core::{Error, Result};
use fs_store::Error as StoreError;
use tokio::sync::oneshot::Sender;

/// The error of a job that ended without an answer.
pub(crate) fn interrupted() -> Error {
    Error::Store(StoreError::Backend(
        "people client: a call ended without an answer".into(),
    ))
}

/// The way back to the caller of a job that is running.
pub(crate) struct Reply<T>(Option<Sender<Result<T>>>);

impl<T> Reply<T> {
    /// The job has started.
    pub(crate) fn started(to: Sender<Result<T>>) -> Self {
        Self(Some(to))
    }

    /// Answer.
    pub(crate) fn send(mut self, answer: Result<T>) {
        if let Some(to) = self.0.take() {
            let _ = to.send(answer);
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot::channel;

    use super::*;

    #[test]
    fn a_job_dropped_after_it_started_answers_that_it_was_interrupted() {
        let (to, mut from) = channel::<Result<u8>>();
        drop(Reply::started(to));
        let got = from.try_recv().unwrap().unwrap_err();
        assert_eq!(got.to_string(), interrupted().to_string());
        assert!(got.to_string().contains("without an answer"), "{got}");
    }

    #[test]
    fn a_job_that_answers_is_not_interrupted() {
        let (to, mut from) = channel::<Result<u8>>();
        Reply::started(to).send(Ok(7));
        assert_eq!(from.try_recv().unwrap().unwrap(), 7);
    }

    #[test]
    fn a_panic_in_the_job_is_an_interruption_not_a_lost_connection() {
        let (to, mut from) = channel::<Result<u8>>();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _reply = Reply::started(to);
            panic!("boom");
        }));
        assert!(caught.is_err());
        let got = from.try_recv().unwrap().unwrap_err();
        assert!(got.to_string().contains("without an answer"), "{got}");
    }
}
