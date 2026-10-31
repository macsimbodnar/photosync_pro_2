id:         S061
goal:       WAITS FOR THE USER AND A LINUX MACHINE: run the GUI on Linux and package it as an AppImage
accepts:    NEEDS THE USER'S LINUX MACHINE; do it in the same sitting as S025 if that is not done yet, as `cargo test --workspace` there covers the GUI's tests too. On Linux, distro and versions in the stamp: `cargo build -p photosync-gui --release`, the GUI's tests pass, with the image tests on loop mounts as the README says, or each failure is its own step; a `scripts/package_linux.sh` makes `target/package/phoosync-pro-2-<arch>.AppImage` from the release binary, a `.desktop` file naming "phoosync pro 2" and `appimagetool`, which is a build tool fetched by the user and not a crate; `ldd` of the binary is in the stamp; the AppImage on a machine without Rust opens, under X11 and Wayland if the user has both, and `--smoke-png` gives a picture, read and described in the stamp, with the fonts seen right, as iced brings Fira Sans and a system font is only for other scripts; the close button during a copy behaves as DEC-069 (4); a real card to a real drive, complete; `sudo`: the sentence, exit 2. Plain tests on Linux are the screen check; the picture and the user's eyes are for the window
touches:    scripts, README; code only by a new step per failure
excludes:   Windows; signing; a .deb or Flatpak
closes:     
paused_by:
author:
done:
