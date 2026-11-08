//! The command line.
//!
//! Pattern: Builder-like parser — plain `std::env::args`, no extra crate for five options.

use std::path::PathBuf;

/// What was asked for on the command line.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// Store file; `None` = in memory, gone when the program ends.
    pub store: Option<PathBuf>,
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
    /// The server's socket (default: the people socket of this user).
    pub socket: Option<PathBuf>,
    /// Keep the people in this process, not at the server (also implied by `--store`, `--import`, `--export`).
    pub local: bool,
    /// `--help` was given.
    pub help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            store: None,
            import: None,
            export: None,
            export_sensitive: Vec::new(),
            snapshot: None,
            size: (1200, 800),
            socket: None,
            local: false,
            help: false,
        }
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
            "--store" => out.store = Some(PathBuf::from(value("--store")?)),
            "--import" => out.import = Some(PathBuf::from(value("--import")?)),
            "--export" => out.export = Some(PathBuf::from(value("--export")?)),
            "--export-sensitive" => out.export_sensitive.push(value("--export-sensitive")?),
            "--snapshot" => out.snapshot = Some(PathBuf::from(value("--snapshot")?)),
            "--size" => out.size = fs_components_app::parse_size(&value("--size")?)?,
            "--socket" => out.socket = Some(PathBuf::from(value("--socket")?)),
            "--local" => out.local = true,
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
pub const HELP: &str = "fs-people: the people window (without arguments it opens)

  --socket <path>    the people server to talk to (default: $XDG_RUNTIME_DIR/freesynergy/people.sock);
                     start it with `fs-people serve`
  --local            keep the people in this process instead of at the server (nothing is shared)
  --store <file>     (local) keep the people in this store file (default: in memory, gone at the end)
  --import <file>    import a vCard file first; people already known are skipped, never replaced
  --export <file>    write everyone to a new vCard file and stop; an existing file is not replaced,
                     people marked sensitive are left out and named
  --export-sensitive <uid>
                     release this person marked sensitive for --export; repeat for each person
  --snapshot <png>   draw a picture instead of opening a window
  --size <WxH>       window or picture size (default 1200x800)";
