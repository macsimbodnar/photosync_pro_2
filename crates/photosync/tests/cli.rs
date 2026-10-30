//! The binary end to end on plain temp dirs. They are all on one device,
//! which strict INV-10 refuses (DEC-025), so no sync succeeds here; the
//! successful syncs are in `exfat_e2e.rs`, on disk images.

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{TempDir, at, manifest, photosync, photosync_tty, write_file};

fn card(root: &Path) {
    write_file(
        root,
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raw",
        at(1_700_000_000),
    );
    write_file(
        root,
        "DCIM/100_FUJI/DSCF0001.JPG",
        b"jpeg",
        at(1_700_000_000),
    );
}

#[test]
fn prints_version() {
    let run = photosync(["--version"]);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.stdout.trim(),
        format!("photosync {}", photosync_core::VERSION)
    );
}

#[test]
fn help_documents_every_command_flag_and_exit_code() {
    let top = photosync(["--help"]);
    assert_eq!(top.code, 0, "{top:?}");
    for part in [
        "sources",
        "drives",
        "sync",
        "With no command, in a terminal, photosync opens its interactive view",
        "Exit codes:",
        "\n  0  ",
        "\n  1  ",
        "\n  2  ",
    ] {
        assert!(
            top.stdout.contains(part),
            "--help lacks {part:?}:\n{}",
            top.stdout
        );
    }
    let sync = photosync(["sync", "--help"]);
    assert_eq!(sync.code, 0, "{sync:?}");
    for part in [
        "--source <PATH>",
        "--dest <LIBRARY>",
        "--name <NAME>",
        "--one-by-one",
        "Exit codes:",
        "\n  2  ",
    ] {
        assert!(
            sync.stdout.contains(part),
            "sync --help lacks {part:?}:\n{}",
            sync.stdout
        );
    }
    for cmd in ["sources", "drives", "sync"] {
        let help = photosync([cmd, "--help"]);
        assert_eq!(help.code, 0, "{help:?}");
        // The test-only override stays out of the help.
        assert!(!help.stdout.contains("search-root"), "{}", help.stdout);
    }
}

#[test]
fn usage_errors_exit_2_and_copy_nothing() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let s = src.to_str().unwrap();
    // No arguments is not here: it opens the TUI, or without a terminal is
    // the usage error of `no_arguments_without_a_terminal_is_a_usage_error`.
    let cases: [&[&str]; 4] = [
        &["frobnicate"],
        &["sync", "--source", s],
        &["sync", "--dest", s],
        &["sync", "--source", s, "--dest"],
    ];
    for args in cases {
        let run = photosync(args);
        assert_eq!(run.code, 2, "{args:?}: {run:?}");
        assert!(run.stdout.is_empty(), "{args:?}: {run:?}");
    }
}

/// With no command the TUI waits for keys; with no terminal nobody can type
/// them. So it must not hang, but print the usage and exit 2, writing
/// nothing. Here stdin is /dev/null and stdout a pipe.
#[test]
fn no_arguments_without_a_terminal_is_a_usage_error() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_photosync"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("photosync with no arguments and no terminal hangs");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(out.stdout.is_empty(), "{stderr}");
    assert!(stderr.contains("needs a terminal"), "{stderr}");
    assert!(stderr.contains("Usage: photosync"), "{stderr}");
}

/// Refused before planning: the library here is missing, which planning
/// would refuse first, and no planning progress shows.
#[cfg(unix)]
#[test]
fn repeated_source_in_any_spelling_is_refused_before_planning() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    std::os::unix::fs::symlink(&src, dir.0.join("link")).unwrap();
    fs::create_dir(dir.0.join("other")).unwrap();
    let library = dir.0.join("missing");
    let spellings = [
        src.clone(),
        src.join("."),
        dir.0.join("other/../card"),
        dir.0.join("link"),
        src.join("DCIM/.."),
    ];
    for again in &spellings {
        let run = photosync([
            "sync".as_ref(),
            "--source".as_ref(),
            src.as_os_str(),
            "--source".as_ref(),
            again.as_os_str(),
            "--dest".as_ref(),
            library.as_os_str(),
        ]);
        assert_eq!(run.code, 2, "{again:?}: {run:?}");
        assert_eq!(
            run.last_err_line(),
            format!(
                "Source {} is the same as source {}. Give each card once.",
                again.display(),
                src.display()
            )
        );
        assert!(!run.stderr.contains("comparing"), "{run:?}");
        assert!(run.stdout.is_empty(), "a report was printed: {run:?}");
    }
    assert!(!library.exists());
}

fn sync(src: &Path, library: &Path) -> common::Run {
    photosync([
        "sync".as_ref(),
        "--source".as_ref(),
        src.as_os_str(),
        "--dest".as_ref(),
        library.as_os_str(),
    ])
}

/// Strict INV-10: one temp disk holds both, so the library is refused,
/// nothing is written, the card is untouched, and no report is printed.
#[test]
fn inv10_library_on_the_same_drive_as_the_source_is_refused() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();
    let before = manifest(&dir.0);

    let run = sync(&src, &library);

    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(
        run.last_err_line(),
        format!(
            "The library {} is on the same drive as source {}. Pick a library folder on \
             another drive (`photosync drives` lists them).",
            library.display(),
            src.display()
        )
    );
    assert!(run.stdout.is_empty(), "a report was printed: {run:?}");
    assert_eq!(manifest(&dir.0), before);
}

#[test]
fn inv10_library_inside_a_source_or_dcim_or_holding_a_source_is_refused() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let other = dir.0.join("other");
    card(&other);
    let cases = [
        (
            src.join("lib"),
            &src,
            format!("is inside source {}", src.display()),
        ),
        (
            other.join("DCIM/lib"),
            &src,
            "is inside a DCIM folder".to_string(),
        ),
        (
            dir.0.clone(),
            &src,
            format!("Source {} is inside the library", src.display()),
        ),
    ];
    for (library, source, part) in cases {
        fs::create_dir_all(&library).unwrap();
        let run = sync(source, &library);
        assert_eq!(run.code, 2, "{library:?}: {run:?}");
        assert!(run.last_err_line().contains(&part), "{run:?}");
        assert!(run.stdout.is_empty(), "{run:?}");
    }
}

/// DEC-028: a missing library is refused, never created.
#[test]
fn missing_library_is_refused_and_not_created() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let library = dir.0.join("drive/Photos");

    let run = sync(&src, &library);

    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.last_err_line().starts_with(&format!(
            "The library folder {} does not exist.",
            library.display()
        )),
        "{run:?}"
    );
    assert!(!dir.0.join("drive").exists());
}

#[test]
fn source_without_dcim_is_refused() {
    let dir = TempDir::new();
    let src = dir.0.join("not_a_card");
    fs::create_dir(&src).unwrap();
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();

    let run = sync(&src, &library);

    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.last_err_line()
            .starts_with(&format!("Cannot read {}", src.join("DCIM").display())),
        "{run:?}"
    );
    assert!(run.stdout.is_empty(), "{run:?}");
}

/// Folders that are not mount points are never volumes (DEC-024), so on
/// a temp dir both lists are empty; `exfat_e2e.rs` lists real mounts.
#[test]
fn sources_and_drives_list_nothing_without_mounted_volumes() {
    let dir = TempDir::new();
    card(&dir.0.join("CARD"));
    fs::create_dir(dir.0.join("SSD")).unwrap();
    let root = dir.0.to_str().unwrap();

    let sources = photosync(["sources", "--search-root", root]);
    let drives = photosync(["drives", "--search-root", root]);

    assert_eq!(sources.code, 0, "{sources:?}");
    assert_eq!(sources.stdout, "");
    assert!(
        sources.stderr.starts_with("No camera card found."),
        "{sources:?}"
    );
    assert_eq!(drives.code, 0, "{drives:?}");
    assert_eq!(drives.stdout, "");
    assert!(
        drives.stderr.starts_with("No drive to sync to found."),
        "{drives:?}"
    );
}

fn one_by_one(src: Option<&Path>, library: &Path, search: &Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec!["sync".into()];
    if let Some(src) = src {
        args.extend(["--source".into(), src.into()]);
    }
    args.extend([
        "--dest".into(),
        library.into(),
        "--one-by-one".into(),
        "--search-root".into(),
        search.into(),
    ]);
    args
}

/// S020: `--one-by-one` asks for the next card on stderr and reads stdin,
/// a terminal. At the end of the input (Ctrl-D) the session ends after
/// its first card instead of hanging. With no `--source` the first card
/// is asked for; none given, nothing synced is not a success. Without a
/// terminal it never asks (S022).
#[test]
fn one_by_one_without_input_ends_after_the_first_card() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();
    let empty = dir.0.join("mounts");
    fs::create_dir(&empty).unwrap();

    let run = photosync_tty(one_by_one(Some(&src), &library, &empty), b"");
    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.stderr.contains("is on the same drive as source"),
        "{run:?}"
    );
    assert_eq!(
        run.last_err_line(),
        "Insert the next card, then press Enter (or type its path). q, or Ctrl-D, finishes."
    );
    assert!(
        run.stdout.is_empty(),
        "one sync, refused, has no report: {run:?}"
    );

    let none = photosync_tty(one_by_one(None, &library, &empty), b"");
    assert_eq!(none.code, 2, "{none:?}");
    assert!(
        none.stderr
            .starts_with("Insert the first card, then press Enter"),
        "{none:?}"
    );
    assert_eq!(none.last_err_line(), "No card was synced.");
}

/// S022: with no terminal on stdin nobody can swap a card, so the prompt
/// never waits, not even on a pipe that stays open: the session ends after
/// the given cards, and with none given nothing was synced.
#[cfg(unix)]
#[test]
fn one_by_one_never_waits_on_a_pipe_that_stays_open() {
    let dir = TempDir::new();
    let src = dir.0.join("card");
    card(&src);
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();
    let empty = dir.0.join("mounts");
    fs::create_dir(&empty).unwrap();

    for (given, last) in [
        (Some(&src), "is on the same drive as source"),
        (None, "No card was synced."),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_photosync"))
            .args(one_by_one(given.map(|p| p.as_path()), &library, &empty))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Held open, never written: a script that keeps its pipe.
        let stdin = child.stdin.take().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                panic!("{given:?}: waits on a pipe that stays open");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let out = child.wait_with_output().unwrap();
        drop(stdin);
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(2), "{given:?}: {stderr}");
        assert!(!stderr.contains("Insert the"), "{given:?}: prompted: {stderr}");
        let last_line = stderr.lines().last().unwrap_or("");
        assert!(last_line.contains(last), "{given:?}: {stderr}");
    }
}

/// S020: one card after another in one session: a typed path, Enter to
/// look for mounted cards (none here, so it asks again), q to finish. Each
/// sync here is refused (one temp device, INV-10): the session's summary
/// names both and the exit code is the session's worst.
#[test]
fn one_by_one_takes_cards_until_q_and_sums_up_the_session() {
    let dir = TempDir::new();
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    card(&a);
    card(&b);
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();
    let empty = dir.0.join("mounts");
    fs::create_dir(&empty).unwrap();
    let before = manifest(&dir.0);

    let input = format!("{}\n\nq\n", b.display());
    let run = photosync_tty(one_by_one(Some(&a), &library, &empty), input.as_bytes());

    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(
        run.stderr.matches("Insert the next card").count(),
        3,
        "{run:?}"
    );
    assert!(run.stderr.contains("\nNo camera card found.\n"), "{run:?}");
    let refused = |card: &Path| {
        format!(
            "The library {} is on the same drive as source {}.",
            library.display(),
            card.display()
        )
    };
    assert!(
        run.stderr.contains(&refused(&a)) && run.stderr.contains(&refused(&b)),
        "{run:?}"
    );
    let summary = [
        "Session of 2 syncs".to_string(),
        format!("Library {}", library.display()),
        "Folder  none (nothing to copy)".to_string(),
        format!("  sync 1 ({}): refused, nothing copied", a.display()),
        format!("  sync 2 ({}): refused, nothing copied", b.display()),
    ];
    assert!(
        run.stdout.starts_with(&(summary.join("\n") + "\n")),
        "{run:?}"
    );
    assert!(
        run.stdout.ends_with(&format!(
            "\nResult: NOT COMPLETE: 2 of 2 syncs not complete: sync 1 ({}): refused, nothing \
             copied; sync 2 ({}): refused, nothing copied. Do not format the cards of those \
             syncs. Fix the cause and sync them again: files already on the drive are skipped.\n",
            a.display(),
            b.display()
        )),
        "{run:?}"
    );
    assert_eq!(manifest(&dir.0), before);
}

/// S020: Ctrl-C while the prompt waits on its terminal ends the session at
/// once, with its summary and its exit code, not by the signal: a second
/// Ctrl-C would.
#[cfg(unix)]
#[test]
fn ctrl_c_at_the_prompt_ends_the_session_with_its_summary() {
    use std::io::{BufRead, BufReader, Write};

    let dir = TempDir::new();
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    card(&a);
    card(&b);
    let library = dir.0.join("library");
    fs::create_dir(&library).unwrap();
    let mut pty = common::Pty::open();
    let mut child = Command::new(env!("CARGO_BIN_EXE_photosync"))
        .args(one_by_one(Some(&a), &library, &dir.0))
        .stdin(pty.stdin())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let stderr = child.stderr.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = tx.send(line.unwrap());
        }
    });
    let prompt = |n: usize| {
        let mut seen = 0;
        while seen < n {
            let line = rx.recv_timeout(Duration::from_secs(30)).expect("no prompt");
            seen += usize::from(line.starts_with("Insert the next card"));
        }
    };
    prompt(1);
    writeln!(pty.keys, "{}", b.display()).unwrap();
    prompt(1);
    // Inside the wait for the answer, not just before it.
    std::thread::sleep(Duration::from_millis(300));
    // SAFETY: kill has no preconditions; the pid is our live child's.
    assert_eq!(
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) },
        0
    );

    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Ctrl-C did not end the session");
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert!(stdout.starts_with("Session of 2 syncs\n"), "{stdout}");
    drop(pty);
}

/// A very deep library takes no folder with a short name in it: the sentence
/// blames the path, as no shorter name rule was broken (DEC-056).
#[test]
fn a_library_too_deep_for_the_folder_is_refused_for_its_path() {
    let dir = TempDir::new();
    let mut library = dir.0.join("library");
    fs::create_dir_all(&library).unwrap();
    while fs::create_dir(library.join("deep-deep")).is_ok() {
        library.push("deep-deep");
    }
    let run = photosync([
        "sync",
        "--source",
        dir.0.join("no-card").to_str().unwrap(),
        "--dest",
        library.to_str().unwrap(),
        "--name",
        "Tirana",
    ]);
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(run.stdout, "", "{run:?}");
    assert!(run.stderr.contains(library.to_str().unwrap()), "{run:?}");
    assert!(run.stderr.contains("too long"), "{run:?}");
    assert!(!run.stderr.contains("242 bytes"), "{run:?}");
    assert!(run.stderr.contains("Nothing was copied."), "{run:?}");
}

/// F06: a name that cannot be a folder name is a usage error, exit 2, before
/// any card is read: the card here does not exist, and is never complained of.
#[test]
fn a_name_that_cannot_be_a_folder_name_is_refused_before_any_card_is_read() {
    let dir = TempDir::new();
    let library = dir.0.join("library");
    fs::create_dir_all(&library).unwrap();
    let long = "a".repeat(300);
    for name in [long.as_str(), "x".repeat(243).as_str(), "a:b", "Tirana."] {
        let run = photosync([
            "sync",
            "--source",
            dir.0.join("no-card").to_str().unwrap(),
            "--dest",
            library.to_str().unwrap(),
            "--name",
            name,
        ]);
        assert_eq!(run.code, 2, "{run:?}");
        assert_eq!(run.stdout, "", "{run:?}");
        assert!(
            run.stderr.starts_with(&format!("The name {name:?} cannot be a folder name.")),
            "{run:?}"
        );
        assert!(run.stderr.contains("242 bytes"), "{run:?}");
        assert!(run.stderr.contains("Nothing was copied."), "{run:?}");
        assert!(!run.stderr.contains("no-card"), "{run:?}");
    }
    assert!(manifest(&library).is_empty());
}
