//! `fs-people`: the command line of people.
//!
//! Pattern: Command — `args` turns words into a [`Command`] (pure), `run` carries one out against a
//! `people::Client` and answers the text to print. The client is whatever the caller connected: the
//! binary connects to the socket, the tests to a server in the same process. Nothing here touches
//! the store; the CLI is a second user of `people.capnp` (the first is the MCP tools).

pub mod args;
mod run;
mod show;

pub use args::{Command, HELP, parse};
pub use run::run;
