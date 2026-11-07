//! The deadlines of the client are the shared ones, and the client still names them.
//!
//! Pattern: Integration tests against the public API; no stand-ins.

use std::time::Duration;

use fs_people_client::{CALL_TIMEOUT, CONNECT_TIMEOUT, Timeouts};

#[test]
fn the_defaults_are_five_seconds_to_connect_and_thirty_per_call() {
    assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(5));
    assert_eq!(CALL_TIMEOUT, Duration::from_secs(30));
    let defaults = Timeouts::default();
    assert_eq!(defaults.connect, CONNECT_TIMEOUT);
    assert_eq!(defaults.call, CALL_TIMEOUT);
}
