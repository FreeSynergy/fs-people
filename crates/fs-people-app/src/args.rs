//! The command line.
//!
//! Pattern: Builder-like parser — plain `std::env::args`, no extra crate for a handful of options.

use std::path::{Path, PathBuf};

/// What was asked for on the command line.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// Unix socket of a running `fs-people-server`; the app holds no store of its own.
    pub socket: Option<PathBuf>,
    /// A `.vcf` file to import before anything else.
    pub import: Option<PathBuf>,
    /// Write everyone to this vCard file; no window opens then.
    pub export: Option<PathBuf>,
    /// The `uid`s of people marked sensitive whom the export may include, each named by the person.
    pub export_sensitive: Vec<String>,
    /// Draw a picture to this file instead of opening a window.
    pub snapshot: Option<PathBuf>,
    /// Size of the window or picture.
    pub size: (u32, u32),
    /// `--help` was given.
    pub help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            socket: None,
            import: None,
            export: None,
            export_sensitive: Vec::new(),
            snapshot: None,
            size: (1200, 800),
            help: false,
        }
    }
}

impl Args {
    /// The socket to connect to.
    ///
    /// # Errors
    /// A message for the person if `--socket` was not given, saying where a socket comes from.
    pub fn socket_path(&self) -> Result<&Path, String> {
        self.socket.as_deref().ok_or_else(|| {
            "no --socket given: the app talks to a running people server; start one with \
             `fs-people-server --socket <path>` and pass the same path here"
                .to_string()
        })
    }
}

/// Parse `args` (without the program name).
///
/// # Errors
/// A message for the person: an unknown option, a missing value or a bad size.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--socket" => out.socket = Some(PathBuf::from(value("--socket")?)),
            "--store" => {
                return Err(
                    "--store is gone: the store belongs to the server, start it with \
                            `fs-people-server --data <dir>` and give the app its --socket"
                        .into(),
                );
            }
            "--import" => out.import = Some(PathBuf::from(value("--import")?)),
            "--export" => out.export = Some(PathBuf::from(value("--export")?)),
            "--export-sensitive" => out.export_sensitive.push(value("--export-sensitive")?),
            "--snapshot" => out.snapshot = Some(PathBuf::from(value("--snapshot")?)),
            "--size" => out.size = fs_components_app::parse_size(&value("--size")?)?,
            "-h" | "--help" => out.help = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    if !out.export_sensitive.is_empty() && out.export.is_none() {
        return Err("--export-sensitive needs --export".into());
    }
    Ok(out)
}

/// The help text.
pub const HELP: &str = "fs-people-app: the people program

  --socket <path>    Unix socket of a running `fs-people-server --socket <path>` (required)
  --import <file>    import a vCard file first; people already known are skipped, never replaced
  --export <file>    write everyone to a new vCard file and stop; an existing file is not replaced,
                     people marked sensitive are left out and named
  --export-sensitive <uid>
                     release this person marked sensitive for --export; repeat for each person
  --snapshot <png>   draw a picture instead of opening a window
  --size <WxH>       window or picture size (default 1200x800)";
