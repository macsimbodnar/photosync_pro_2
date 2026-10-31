//! How a test compares a picture, for the unit tests and `smoke_window.rs` both.

use std::fs;
use std::path::{Path, PathBuf};

use iced_test::simulator::Snapshot;

fn pictures_in(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "png"))
        .collect()
}

/// `snapshot` is the one picture in `dir`. `matches_image` adds the renderer's name to
/// `name`: with no file of that name it writes one and says they match, having compared
/// nothing, so another renderer would pass. Then the folder holds a second picture.
pub fn same_picture(snapshot: &Snapshot, dir: &Path, name: &str) -> Result<(), String> {
    let before = pictures_in(dir);
    if before.len() != 1 {
        return Err(format!("not one picture to compare with: {before:?}"));
    }
    let same = snapshot
        .matches_image(dir.join(format!("{name}.png")))
        .map_err(|e| e.to_string())?;
    let after = pictures_in(dir);
    if after.len() != 1 {
        return Err(format!(
            "a picture of another name was written, so this compared nothing: {after:?}"
        ));
    }
    if !same {
        return Err("the pixels differ".to_string());
    }
    Ok(())
}
