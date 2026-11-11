//! The worker thread: owns the connection, carries out the calls.
//!
//! Pattern: Command — each `Call` is carried out against the server's capabilities, one task each.

use std::sync::Arc;
use std::time::Duration;

use fs_interfaces::people_capnp::{people, persons, roles};
use fs_interfaces::rpc::client::call_limited;
use tokio::sync::mpsc;

use super::call::{Call, Revisions};
use super::dial::Dial;
use super::limits::{Limits, SERVER, timed_out, within};
use super::persons::{create, delete, get, list, update, update_if};
use super::roles::{role, roles_of};

pub(super) fn worker(
    dial: &Dial,
    limits: Limits,
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
        let connection = match dial.connect().await {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(e));
                return;
            }
        };
        // The first questions have a limit too: a server that passed the handshake and then says nothing must not keep
        // the window from opening.
        let (persons, roles) =
            match call_limited(SERVER, limits.start, open(&connection.client)).await {
                Ok(Ok(both)) => both,
                Ok(Err(e)) => {
                    let _ = ready.send(Err(e.to_string()));
                    return;
                }
                Err(fault) => {
                    let _ = ready.send(Err(timed_out(&fault).to_string()));
                    return;
                }
            };
        let _ = ready.send(Ok(()));
        while let Some(call) = inbox.recv().await {
            let (persons, roles, revisions) =
                (persons.clone(), roles.clone(), Arc::clone(revisions));
            tokio::task::spawn_local(async move {
                carry_out(&persons, &roles, &revisions, limits.call, call).await;
            });
        }
    });
}

async fn carry_out(
    persons: &persons::Client,
    roles: &roles::Client,
    revisions: &Revisions,
    limit: Duration,
    call: Call,
) {
    match call {
        Call::RolesOf(id, reply) => {
            let _ = reply.send(within(limit, roles_of(roles, id)).await);
        }
        Call::Role(id, reply) => {
            let _ = reply.send(within(limit, role(roles, &id)).await);
        }
        Call::Get(id, reply) => {
            let _ = reply.send(within(limit, get(persons, revisions, id)).await);
        }
        Call::List(filter, reply) => {
            let _ = reply.send(within(limit, list(persons, revisions, &filter)).await);
        }
        Call::Create(text, reply) => {
            let _ = reply.send(within(limit, create(persons, revisions, &text)).await);
        }
        Call::Update(id, text, reply) => {
            let _ = reply.send(within(limit, update(persons, revisions, id, &text)).await);
        }
        Call::UpdateIf(id, revision, text, reply) => {
            let _ =
                reply.send(within(limit, update_if(persons, revisions, id, revision, &text)).await);
        }
        Call::Delete(id, reply) => {
            let _ = reply.send(within(limit, delete(persons, id)).await);
        }
    }
}

async fn open(root: &people::Client) -> capnp::Result<(persons::Client, roles::Client)> {
    let persons = root
        .persons_request()
        .send()
        .promise
        .await?
        .get()?
        .get_persons()?;
    let roles = root
        .roles_request()
        .send()
        .promise
        .await?
        .get()?
        .get_roles()?;
    Ok((persons, roles))
}
