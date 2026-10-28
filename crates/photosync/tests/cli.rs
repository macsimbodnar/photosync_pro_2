//! The binary end to end on plain temp dirs. They are all on one device,
//! which strict INV-10 refuses (DEC-025), so no sync succeeds here; the
//! successful syncs are in `exfat_e2e.rs`, run by `scripts/test_exfat.sh`.

mod common;

use std::fs;
use std::path::Path;

use common::{TempDir, at, manifest, photosync, write_file};

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
        "Exit codes:",
        "\n  2  ",
    ] {
        assert!(
            sync.stdout.contains(part),
            "sync --help lacks {part:?}:\n{}",
            sync.stdout
        );
    }
    for cmd in ["sources", "drives"] {
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
    let cases: [&[&str]; 5] = [
        &[],
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
