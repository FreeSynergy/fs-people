//! `People` over the socket of the people server, split by responsibility.
//!
//! Pattern: Proxy — `proxy.rs` is the proxy, `worker.rs` the thread that owns the connection,
//! `call.rs` the calls between them, `persons.rs`/`roles.rs` the requests, `convert.rs` the
//! conversions, `no_roles.rs` the empty role book.

mod call;
mod convert;
mod no_roles;
mod persons;
mod proxy;
mod roles;
mod worker;

pub use no_roles::NoRoles;
pub use proxy::RemotePeople;
