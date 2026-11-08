//! The command line as a function of the one program (ADR 0063).
//!
//! Pattern: Facade — parse, connect to the socket, run one command, print. `fs-people <command>`
//! calls [`command_line`]; the exit code is the answer.

use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{connect, default_socket};

use crate::{HELP, parse, run};

/// Carry out the words after `fs-people` (a command, maybe with `--socket`). The exit code: 0 done,
/// 1 the server or the system refused, 2 the words were wrong.
#[must_use]
pub fn command_line(words: &[String]) -> i32 {
    let args = match parse(words) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fs-people: {e}");
            return 2;
        }
    };
    let Some(command) = args.command else {
        println!("{HELP}");
        return 0;
    };
    let Some(socket) = args.socket.or_else(|| default_socket("people")) else {
        eprintln!("fs-people: no socket: give --socket or set XDG_RUNTIME_DIR");
        return 2;
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fs-people: {e}");
            return 1;
        }
    };
    let local = tokio::task::LocalSet::new();
    let out = local.block_on(&runtime, async move {
        let connection = connect::<people::Client>(&socket)
            .await
            .map_err(|e| format!("{}: {e}", socket.display()))?;
        run(&connection.client, command).await
    });
    match out {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(e) => {
            eprintln!("fs-people: {e}");
            1
        }
    }
}
