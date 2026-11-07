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
