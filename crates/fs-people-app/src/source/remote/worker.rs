//! The worker thread: owns the connection, carries out the calls.
//!
//! Pattern: Command — each `Call` is carried out against the server's capabilities, one task each.

use std::path::Path;
use std::sync::Arc;

use fs_interfaces::people_capnp::{people, persons, roles};
use fs_interfaces::rpc::connect;
use tokio::sync::mpsc;

use super::call::{Call, Revisions};
use super::persons::{create, delete, get, list, update, update_if};
use super::roles::{role, roles_of};

pub(super) fn worker(
    socket: &Path,
    mut inbox: mpsc::UnboundedReceiver<Call>,
    ready: &std::sync::mpsc::Sender<std::result::Result<(), String>>,
    revisions: &Revisions,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let connection = match connect::<people::Client>(socket).await {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(format!(
                    "no people server on {}: {e}; start it with `fs-people serve`",
                    socket.display()
                )));
                return;
            }
        };
        let persons = match connection.client.persons_request().send().promise.await {
            Ok(r) => match r
                .get()
                .and_then(people::persons_results::Reader::get_persons)
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = ready.send(Err(e.to_string()));
                    return;
                }
            },
            Err(e) => {
                let _ = ready.send(Err(e.to_string()));
                return;
            }
        };
        let roles = match connection.client.roles_request().send().promise.await {
            Ok(r) => match r.get().and_then(people::roles_results::Reader::get_roles) {
                Ok(p) => p,
                Err(e) => {
                    let _ = ready.send(Err(e.to_string()));
                    return;
                }
            },
            Err(e) => {
                let _ = ready.send(Err(e.to_string()));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        while let Some(call) = inbox.recv().await {
            let (persons, roles, revisions) =
                (persons.clone(), roles.clone(), Arc::clone(revisions));
            tokio::task::spawn_local(async move {
                carry_out(&persons, &roles, &revisions, call).await;
            });
        }
    });
}

async fn carry_out(
    persons: &persons::Client,
    roles: &roles::Client,
    revisions: &Revisions,
    call: Call,
) {
    match call {
        Call::RolesOf(id, reply) => {
            let _ = reply.send(roles_of(roles, id).await);
        }
        Call::Role(id, reply) => {
            let _ = reply.send(role(roles, &id).await);
        }
        Call::Get(id, reply) => {
            let _ = reply.send(get(persons, revisions, id).await);
        }
        Call::List(filter, reply) => {
            let _ = reply.send(list(persons, revisions, &filter).await);
        }
        Call::Create(text, reply) => {
            let _ = reply.send(create(persons, revisions, &text).await);
        }
        Call::Update(id, text, reply) => {
            let _ = reply.send(update(persons, revisions, id, &text).await);
        }
        Call::UpdateIf(id, revision, text, reply) => {
            let _ = reply.send(update_if(persons, revisions, id, revision, &text).await);
        }
        Call::Delete(id, reply) => {
            let _ = reply.send(delete(persons, id).await);
        }
    }
}
