//! Every playbook in `zawnabout-playbooks`, run headless against the real binary, several
//! at once. They need a GPU (a software one does), so they only build with the
//! `playbooks` feature:
//!
//! ```text
//! cargo test --release -p zawnabout-native --features playbooks --test playbooks -- --nocapture
//! ```
//!
//! Each one's log and shots go under `ZAWNABOUT_PLAYBOOK_OUT` if set, else under the
//! target directory. `ZAWNABOUT_PLAYBOOKS` picks some by name, comma separated, instead of
//! running them all.

#![cfg(feature = "playbooks")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

#[test]
fn every_playbook_passes() {
    let projects = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut books: Vec<PathBuf> = std::fs::read_dir(projects.join("zawnabout-playbooks"))
        .expect("read the playbooks")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    books.sort();
    assert!(!books.is_empty(), "no playbooks found");
    let stem = |p: &PathBuf| p.file_stem().map(|s| s.to_string_lossy().into_owned());
    if let Ok(pick) = std::env::var("ZAWNABOUT_PLAYBOOKS")
        && !pick.trim().is_empty()
    {
        let names: Vec<&str> = pick.split(',').map(str::trim).collect();
        for name in &names {
            assert!(
                books.iter().any(|b| stem(b).as_deref() == Some(name)),
                "no playbook named {name}"
            );
        }
        books.retain(|b| stem(b).is_some_and(|s| names.contains(&s.as_str())));
    }
    let out = std::env::var_os("ZAWNABOUT_PLAYBOOK_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_TARGET_TMPDIR")).join("playbooks"));
    std::fs::create_dir_all(&out).expect("make the output folder");

    // One per core, each taking the next playbook in the list until none are left.
    let queue = Mutex::new(books.iter());
    let failed = Mutex::new(Vec::new());
    let jobs = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(books.len());
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| {
                loop {
                    let Some(book) = queue.lock().expect("queue").next() else {
                        return;
                    };
                    let name = book.file_stem().expect("a name").to_string_lossy();
                    let start = Instant::now();
                    let run = Command::new(env!("CARGO_BIN_EXE_zawnabout"))
                        .arg("--headless")
                        .arg("--script")
                        .arg(book)
                        .arg("--out")
                        .arg(out.join(&*name))
                        .current_dir(&projects)
                        .output()
                        .expect("run the game");
                    let log = String::from_utf8_lossy(&run.stdout).into_owned()
                        + &String::from_utf8_lossy(&run.stderr);
                    std::fs::write(out.join(format!("{name}.log")), &log).expect("write the log");
                    let secs = start.elapsed().as_secs();
                    if run.status.success() {
                        println!("ok   {name} ({secs}s)");
                    } else {
                        println!("FAIL {name} ({secs}s)");
                        let tail: Vec<&str> = log.lines().rev().take(5).collect();
                        let tail: Vec<&str> = tail.into_iter().rev().collect();
                        failed
                            .lock()
                            .expect("failures")
                            .push(format!("{name}:\n  {}", tail.join("\n  ")));
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().expect("failures");
    assert!(
        failed.is_empty(),
        "{} of {} playbooks failed (logs in {}):\n{}",
        failed.len(),
        books.len(),
        out.display(),
        failed.join("\n")
    );
}
