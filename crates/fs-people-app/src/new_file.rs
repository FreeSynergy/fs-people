//! Writing a new file without ever replacing one or leaving half of one.
//!
//! Pattern: Template method — fixed order: a temporary file beside the target, the whole content,
//! a sync, then the target appears at once through a hard link, which refuses a name that is taken
//! (a rename would replace it). Where the file system has no hard links, the target is created
//! directly from the temporary file, and removed again if that fails. Only files this code created
//! are ever removed. The temporary name is short, so a long target name still fits.

use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

/// Temporary names tried before giving up.
const ATTEMPTS: u32 = 16;

/// Write `bytes` to `path`, which must not exist yet.
///
/// # Errors
/// A message for the person; nothing is left behind.
pub(crate) fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let steps = Steps {
        fill: &|file: &mut File| file.write_all(bytes),
        link: &|from: &Path, to: &Path| fs::hard_link(from, to),
        copy: &|from: &mut File, to: &mut File| io::copy(from, to).map(|_| ()),
    };
    write_with(path, &steps).map_err(|e| format!("cannot create {}: {e}", path.display()))
}

type Fill<'a> = &'a dyn Fn(&mut File) -> io::Result<()>;
type Link<'a> = &'a dyn Fn(&Path, &Path) -> io::Result<()>;
type Copy<'a> = &'a dyn Fn(&mut File, &mut File) -> io::Result<()>;

/// The steps that can fail on a real disk, so a test can make them fail.
struct Steps<'a> {
    fill: Fill<'a>,
    link: Link<'a>,
    copy: Copy<'a>,
}

fn write_with(path: &Path, steps: &Steps<'_>) -> io::Result<()> {
    if path.exists() {
        return Err(io::Error::new(ErrorKind::AlreadyExists, "the file exists"));
    }
    let (temporary, mut file) = new_temporary(path)?;
    let done = fill_and_place(&mut file, &temporary, path, steps);
    let _ = fs::remove_file(&temporary);
    done
}

/// A new file beside `path`. A file of that name that was already there is left alone.
fn new_temporary(path: &Path) -> io::Result<(PathBuf, File)> {
    let mut last = io::Error::new(ErrorKind::AlreadyExists, "no free temporary name");
    for attempt in 0..ATTEMPTS {
        let name = format!(".fsp{}-{attempt}.tmp", std::process::id());
        let temporary = path.with_file_name(name);
        match File::create_new(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

fn fill_and_place(
    file: &mut File,
    temporary: &Path,
    path: &Path,
    steps: &Steps<'_>,
) -> io::Result<()> {
    (steps.fill)(file)?;
    file.sync_all()?;
    match (steps.link)(temporary, path) {
        Err(e) if e.kind() != ErrorKind::AlreadyExists => write_directly(temporary, path, steps),
        other => other,
    }
}

/// The file system cannot link: make the target from the finished temporary file.
fn write_directly(temporary: &Path, path: &Path, steps: &Steps<'_>) -> io::Result<()> {
    let mut target = File::create_new(path)?;
    let copied = File::open(temporary)
        .and_then(|mut from| (steps.copy)(&mut from, &mut target))
        .and_then(|()| target.sync_all());
    if copied.is_err() {
        // Created just above, so ours to remove; otherwise a retry would find it taken.
        let _ = fs::remove_file(path);
    }
    copied
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fs-new-file-{}", fs_store::ObjectId::new()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn steps_with<'a>(fill: Fill<'a>, link: Link<'a>, copy: Copy<'a>) -> Steps<'a> {
        Steps { fill, link, copy }
    }

    fn writes(file: &mut File) -> io::Result<()> {
        file.write_all(b"whole")
    }

    fn links(from: &Path, to: &Path) -> io::Result<()> {
        fs::hard_link(from, to)
    }

    fn no_link(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::new(ErrorKind::Unsupported, "no hard links here"))
    }

    fn copies(from: &mut File, to: &mut File) -> io::Result<()> {
        io::copy(from, to).map(|_| ())
    }

    #[test]
    fn a_failed_write_leaves_nothing_behind() {
        let dir = folder();
        let fails = |file: &mut File| {
            file.write_all(b"half")?;
            Err(io::Error::other("the disk is full"))
        };
        let got = write_with(&dir.join("out.vcf"), &steps_with(&fails, &links, &copies));
        assert!(got.is_err());
        assert!(names(&dir).is_empty(), "no target, no temporary file");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_file_appears_whole_and_a_taken_name_is_not_replaced() {
        let dir = folder();
        let path = dir.join("out.vcf");
        write_new_file(&path, b"one").unwrap();
        assert!(write_new_file(&path, b"two").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"one");
        assert_eq!(names(&dir), ["out.vcf"], "no temporary file");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn without_hard_links_the_target_comes_whole_from_the_finished_file() {
        let dir = folder();
        let path = dir.join("out.vcf");
        write_with(&path, &steps_with(&writes, &no_link, &copies)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"whole");
        assert_eq!(names(&dir), ["out.vcf"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_direct_write_that_fails_removes_its_half_file_so_a_retry_works() {
        let dir = folder();
        let path = dir.join("out.vcf");
        let half = |_: &mut File, to: &mut File| {
            to.write_all(b"ha")?;
            Err(io::Error::other("the disk is full"))
        };
        assert!(write_with(&path, &steps_with(&writes, &no_link, &half)).is_err());
        assert!(names(&dir).is_empty(), "{:?}", names(&dir));
        write_with(&path, &steps_with(&writes, &no_link, &copies)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"whole");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_with_the_temporary_name_is_not_ours_and_stays() {
        let dir = folder();
        let foreign = dir.join(format!(".fsp{}-0.tmp", std::process::id()));
        fs::write(&foreign, b"someone else's").unwrap();
        write_new_file(&dir.join("out.vcf"), b"one").unwrap();
        assert_eq!(fs::read(&foreign).unwrap(), b"someone else's");
        assert_eq!(fs::read(dir.join("out.vcf")).unwrap(), b"one");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_free_temporary_name_deletes_nothing() {
        let dir = folder();
        for attempt in 0..ATTEMPTS {
            let name = format!(".fsp{}-{attempt}.tmp", std::process::id());
            fs::write(dir.join(name), b"x").unwrap();
        }
        assert!(write_new_file(&dir.join("out.vcf"), b"one").is_err());
        assert_eq!(
            names(&dir).len(),
            ATTEMPTS as usize,
            "all still there, no target"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_target_name_as_long_as_the_file_system_allows_works() {
        let dir = folder();
        let long = dir.join("x".repeat(255));
        write_new_file(&long, b"one").unwrap();
        assert_eq!(fs::read(&long).unwrap(), b"one");
        fs::remove_dir_all(dir).unwrap();
    }
}
