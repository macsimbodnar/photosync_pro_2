//! Disk images the tests mount for themselves (macOS, through hdiutil), so
//! plain `cargo test` proves real syncs: strict INV-10 lets a sync through
//! only across devices, and the binary has no seam to fake one (DEC-027).
//! Both crates' tests share this file. `scripts/test_exfat.sh` mounts its
//! own images and leaves out the tests that use this.
//!
//! The tests that need mounts are `#[ignore]`d, so a run without them shows
//! in the count. One plain test in each test binary mounts the images and
//! runs that same binary again with `--ignored`. They are never attached
//! longer than that test, whatever happens to it:
//! - a panic detaches them (`Drop`);
//! - a kill or a Ctrl-C of the test binary detaches them (the watchdog);
//! - every command has a time limit, the watchdog's too, as macOS's disk-image
//!   service can hang. A watchdog that gives up says so in `watchdog.log` in
//!   the temp folder, and leaves the folder.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub const CARD: &str = "PHOTOSYNC_TEST_CARD";
pub const LIBRARY: &str = "PHOTOSYNC_TEST_LIBRARY";
pub const TWIN_A: &str = "PHOTOSYNC_TEST_TWIN_A";
pub const TWIN_B: &str = "PHOTOSYNC_TEST_TWIN_B";

/// Well above what each takes (about 1 s, and 10 s for a binary's tests): a
/// limit is there to end a hang, never to fail a slow run.
const CREATE_LIMIT: Duration = Duration::from_secs(120);
const DETACH_LIMIT: Duration = Duration::from_secs(30);
const INFO_LIMIT: Duration = Duration::from_secs(30);
const TESTS_LIMIT: Duration = Duration::from_secs(300);

const HUNG: &str = "macOS's disk-image service may be hung: see `hdiutil info`; a restart \
                    clears it";

/// The variable, its volume name, size and mount point's name. The first
/// is the temp dir, on a device of its own like a real card.
const IMAGES: [(&str, &str, &str, &str); 5] = [
    ("TMPDIR", "PSEXFAT", "64m", "mnt"),
    (CARD, "PSCARD", "16m", "card"),
    (LIBRARY, "PSLIB", "16m", "lib"),
    (TWIN_A, "PSTWINA", "16m", "twin_a"),
    (TWIN_B, "PSTWINB", "16m", "twin_b"),
];

/// What the folder holding the mounts starts with: `Images::mount`'s, and the script's.
const WORK_FOLDERS: [&str; 2] = ["photosync-images-", "photosync-exfat."];

/// Waits on stdin, which only this process holds. "done" means the images
/// are dealt with. The end of the input with no "done" means the test binary
/// died with them attached, as no `Drop` runs on a kill, so its tests go and
/// the images are detached. Its own process group, so a Ctrl-C at the
/// terminal does not end it too.
const WATCHDOG: &str = r#"
work=$1
limit=$2
shift 2
group=
while read -r word value; do
    case $word in
        group) group=$value ;;
        done) exit 0 ;;
    esac
done
[ -n "$group" ] && kill -9 "-$group" 2>/dev/null

# To a file: its stderr is /dev/null, as it can outlive what reads the binary's.
say() {
    echo "$*" >>"$work/watchdog.log"
}

# Stopped first, so it can neither end well when a helper dies nor start another.
# Killed last, so whoever waits for it wakes only when the whole tree is gone.
kill_tree() {
    kill -STOP "$1" 2>/dev/null
    for pid in $(pgrep -P "$1"); do
        kill_tree "$pid"
    done
    kill -9 "$1" 2>/dev/null
}

# Runs "$@" within the limit (macOS has no `timeout`), its output in $out: a
# pipe could stay open after it ends, in a helper it started. Its status.
out=$work/watchdog.out
limited() {
    "$@" >"$out" 2>/dev/null </dev/null &
    running=$!
    ( sleep "$limit"; kill_tree "$running" ) </dev/null >/dev/null 2>&1 &
    watcher=$!
    wait "$running"
    status=$?
    kill_tree "$watcher"
    wait "$watcher" 2>/dev/null
    # 137: killed with -9, and only the watcher does that here.
    [ "$status" -eq 137 ] && say "$* did not end in ${limit}s and was killed."
    return "$status"
}

# Whole-disk devices of attached images under $work, from `hdiutil info`.
mine() {
    awk -v dir="$work/" '
        /^image-path/ { sub(/^image-path *: */, ""); mine = (index($0, dir) == 1); next }
        mine && /^\/dev\/disk[0-9]+[ \t]/ { print $1; mine = 0 }
    ' "$out"
}

for point in "$@"; do
    limited hdiutil detach "$point" -quiet ||
        limited hdiutil detach "$point" -force -quiet
done
# Attached but not mounted: an attach killed midway.
if limited hdiutil info; then
    for dev in $(mine); do
        limited hdiutil detach "$dev" -force -quiet
    done
fi
# Removed only if both show nothing left: rm through a live mount would delete
# inside the image. A check that fails or hangs counts as something left.
if limited hdiutil info && [ -z "$(mine)" ] && limited mount; then
    case $(cat "$out") in
        *" on $work/"*) ;;
        *)
            rm -rf "$work"
            exit 0
            ;;
    esac
fi
say "left $work in place. Detach by hand: hdiutil detach <each> -force; rm -rf '$work'." \
    "macOS's disk-image service may be hung: see 'hdiutil info'; a restart clears it."
"#;

/// A command running with its output in a file: a pipe could stay open
/// after it ends, in a helper it started, and block the reader for ever.
pub struct Logged {
    child: Child,
    log: PathBuf,
}

/// Starts `cmd` in its own process group, so a time limit can end what it
/// started too, with no terminal: a background group reading one would stop.
pub fn start(cmd: &mut Command, log_dir: &Path) -> Result<Logged, String> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let log = log_dir.join(format!(
        "command-{}.log",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let what = format!("{cmd:?}");
    let out = File::create(&log).map_err(|e| format!("Cannot create {}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("Cannot clone a log file: {e}"))?;
    let child = cmd
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| format!("Cannot run {what}: {e}"))?;
    Ok(Logged { child, log })
}

impl Logged {
    /// Its exit status and output. `Err`, naming `what`, if it runs past
    /// `limit`: then its whole process group is killed.
    pub fn finish(mut self, limit: Duration, what: &str) -> Result<(ExitStatus, String), String> {
        let start = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    let output = fs::read(&self.log).unwrap_or_default();
                    return Ok((status, String::from_utf8_lossy(&output).into_owned()));
                }
                Ok(None) => {}
                Err(e) => return Err(format!("Cannot wait for {what}: {e}")),
            }
            if start.elapsed() > limit {
                let group = self.child.id() as libc::pid_t;
                // SAFETY: kill takes two plain ints. The group is this
                // child's own, and it is not reaped yet, so the id is not
                // reused.
                unsafe { libc::kill(-group, libc::SIGKILL) };
                let output = fs::read(&self.log).unwrap_or_default();
                return Err(format!(
                    "{what} did not end in {}s and was killed. Its output:\n{}",
                    limit.as_secs(),
                    String::from_utf8_lossy(&output)
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Runs `cmd` to its end within `limit`. `Err` on a failure too. Its output.
pub fn limited(
    cmd: &mut Command,
    limit: Duration,
    what: &str,
    log_dir: &Path,
) -> Result<String, String> {
    let (status, output) = start(cmd, log_dir)?.finish(limit, what)?;
    if !status.success() {
        return Err(format!("{what} failed ({status}):\n{output}"));
    }
    Ok(output)
}

/// Whole-disk devices of attached images whose file is under `dir`.
pub fn attached_under(dir: &Path, log_dir: &Path) -> Result<Vec<String>, String> {
    let info = limited(
        Command::new("hdiutil").arg("info"),
        INFO_LIMIT,
        "hdiutil info",
        log_dir,
    )?;
    let dir = format!("{}/", dir.display());
    let mut mine = false;
    let mut devices = Vec::new();
    for line in info.lines() {
        if let Some(rest) = line.strip_prefix("image-path") {
            let path = rest.trim_start().trim_start_matches(':').trim_start();
            mine = path.starts_with(&dir);
        } else if mine && line.starts_with("/dev/disk") {
            let dev = line.split_whitespace().next().unwrap_or("");
            // The first line is the whole disk, the rest its partitions.
            if dev["/dev/disk".len()..].chars().all(|c| c.is_ascii_digit()) {
                devices.push(dev.to_string());
                mine = false;
            }
        }
    }
    Ok(devices)
}

/// `Err` naming what is missing, before anything is made. Without these
/// nothing was attached, so a failure to detach would be a false alarm.
fn check_tools() -> Result<(), String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let on_path = |tool: &&str| {
        std::env::split_paths(&path).any(|dir| {
            fs::metadata(dir.join(tool))
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
    };
    let missing: Vec<&str> = ["hdiutil", "mount"]
        .into_iter()
        .filter(|tool| !on_path(tool))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "The image tests need {} on PATH, which is {path:?}.",
        missing.join(" and ")
    ))
}

/// The images under one temp folder, mounted side by side, as `/Volumes`
/// has them. Detached on drop, on a panic too; call `detach` to see a
/// failure to detach as a failed test.
pub struct Images {
    work: PathBuf,
    env: Vec<(&'static str, PathBuf)>,
    watchdog: Option<(Child, ChildStdin)>,
    finished: bool,
}

impl Images {
    /// The temp dir's image and those of `vars`, all exFAT, with `DCIM` at
    /// the top of the card. Nothing is left attached on an `Err`.
    pub fn mount(vars: &[&str]) -> Result<Images, String> {
        check_tools()?;
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let work = std::env::temp_dir().join(format!(
            "photosync-images-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&work).map_err(|e| format!("Cannot create {}: {e}", work.display()))?;
        // Resolved, so it matches what `mount` prints (/var is /private/var).
        let work = fs::canonicalize(&work).map_err(|e| format!("{}: {e}", work.display()))?;
        let watchdog = start_watchdog(&work).inspect_err(|_| {
            let _ = fs::remove_dir(&work);
        })?;
        let mut images = Images {
            watchdog: Some(watchdog),
            work,
            env: Vec::new(),
            finished: false,
        };
        for (var, volume, size, point) in IMAGES {
            if var == "TMPDIR" || vars.contains(&var) {
                let point = images.attach(volume, size, point)?;
                if var == CARD {
                    let dcim = point.join("DCIM");
                    fs::create_dir(&dcim).map_err(|e| format!("{}: {e}", dcim.display()))?;
                }
                images.env.push((var, point));
            }
        }
        Ok(images)
    }

    pub fn work(&self) -> &Path {
        &self.work
    }

    fn attach(&self, volume: &str, size: &str, point: &str) -> Result<PathBuf, String> {
        let image = self.work.join(format!("{volume}.dmg"));
        let path = self.work.join(point);
        fs::create_dir(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let hung = |e: String| format!("{e}\n{HUNG}.");
        limited(
            Command::new("hdiutil")
                .args([
                    "create", "-size", size, "-fs", "ExFAT", "-volname", volume, "-ov", "-quiet",
                ])
                .arg(&image),
            CREATE_LIMIT,
            &format!("hdiutil create {volume}"),
            &self.work,
        )
        .map_err(hung)?;
        limited(
            Command::new("hdiutil")
                .arg("attach")
                .arg(&image)
                .args(["-nobrowse", "-quiet", "-mountpoint"])
                .arg(&path),
            CREATE_LIMIT,
            &format!("hdiutil attach {volume}"),
            &self.work,
        )
        .map_err(hung)?;
        let mounts = self.mount_table()?;
        if !mounts.contains(&format!(" on {} (exfat", path.display())) {
            return Err(format!(
                "{} is not mounted as exFAT:\n{mounts}",
                path.display()
            ));
        }
        Ok(path)
    }

    fn mount_table(&self) -> Result<String, String> {
        limited(&mut Command::new("mount"), INFO_LIMIT, "mount", &self.work)
    }

    /// Runs this test binary again with `--ignored` on these images. Every
    /// test it lists must pass, and `must_run` must be among them.
    fn run_ignored_tests(&self, must_run: &[&str]) -> Result<usize, String> {
        let exe = std::env::current_exe().map_err(|e| format!("No test binary path: {e}"))?;
        let list = limited(
            Command::new(&exe).args(["--list", "--ignored"]),
            INFO_LIMIT,
            "the list of ignored tests",
            &self.work,
        )?;
        let listed: Vec<&str> = list
            .lines()
            .filter_map(|l| l.strip_suffix(": test"))
            .collect();
        let missing: Vec<&&str> = must_run.iter().filter(|t| !listed.contains(t)).collect();
        if !missing.is_empty() {
            return Err(format!("These tests are gone or not ignored: {missing:?}"));
        }
        let running = start(
            Command::new(&exe)
                .arg("--ignored")
                .envs(self.env.iter().cloned()),
            &self.work,
        )?;
        self.tell_watchdog(&format!("group {}\n", running.child.id()));
        let (status, output) = running.finish(TESTS_LIMIT, "the tests on the images")?;
        self.tell_watchdog("group\n");
        // A count of zero that passes proves nothing, and a name filter
        // left in here could make one.
        let want = format!(
            "test result: ok. {} passed; 0 failed; 0 ignored",
            listed.len()
        );
        if !status.success() || !output.contains(&want) {
            return Err(format!(
                "The tests on the images failed ({status}; wanted \"{want}\"):\n{output}"
            ));
        }
        Ok(listed.len())
    }

    fn tell_watchdog(&self, line: &str) {
        if let Some((_, mut stdin)) = self.watchdog.as_ref().map(|(c, s)| (c, s)) {
            // Failing means the watchdog is gone. `Drop` still detaches; only
            // a later kill of this binary would leave images attached.
            let _ = stdin.write_all(line.as_bytes());
        }
    }

    /// Detaches the images and removes the temp folder. `Err` names what is
    /// still attached, and then the folder stays.
    pub fn detach(mut self) -> Result<(), String> {
        self.finished = true;
        self.clean_up()
    }

    fn clean_up(&mut self) -> Result<(), String> {
        let mut left = Vec::new();
        for (_, _, _, point) in IMAGES {
            let path = self.work.join(point);
            let needle = format!(" on {} (", path.display());
            // Every mount point, listed for this set or not: an attach may
            // have mounted before it failed.
            if self.mount_table().is_ok_and(|m| m.contains(&needle)) {
                self.detach_one(path.as_os_str());
            }
            // Checked again, not trusted to the detach: removing the folder
            // through a live mount would delete inside the image. A failing
            // `mount` counts as still mounted, so this fails safe.
            if self.mount_table().is_ok_and(|m| !m.contains(&needle)) {
                continue;
            }
            left.push(path.display().to_string());
        }
        // Attached but not mounted: an attach killed midway.
        if let Ok(devices) = attached_under(&self.work, &self.work) {
            for dev in &devices {
                self.detach_one(dev.as_ref());
            }
        }
        match attached_under(&self.work, &self.work) {
            Ok(devices) => left.extend(devices),
            Err(e) => left.push(e),
        }
        if let Some((mut child, mut stdin)) = self.watchdog.take() {
            let _ = stdin.write_all(b"done\n");
            drop(stdin);
            // It ends at once. Not waited for without a limit: nothing here
            // may hang, and a zombie is gone with this binary.
            let start = Instant::now();
            while matches!(child.try_wait(), Ok(None)) && start.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if !left.is_empty() {
            return Err(format!(
                "FAILED to detach {}; left them and {} in place. Detach by hand: hdiutil detach \
                 <each> -force; rm -rf '{}'. {HUNG}.",
                left.join(", "),
                self.work.display(),
                self.work.display()
            ));
        }
        fs::remove_dir_all(&self.work).map_err(|e| format!("{}: {e}", self.work.display()))
    }

    fn detach_one(&self, what: &std::ffi::OsStr) {
        for flags in [&["-quiet"][..], &["-force", "-quiet"]] {
            let done = limited(
                Command::new("hdiutil").arg("detach").arg(what).args(flags),
                DETACH_LIMIT,
                "hdiutil detach",
                &self.work,
            );
            if done.is_ok() {
                return;
            }
        }
    }
}

impl Drop for Images {
    fn drop(&mut self) {
        if !self.finished
            && let Err(e) = self.clean_up()
        {
            eprintln!("{e}");
        }
    }
}

/// The watchdog, not started. Each command it runs gets `limit`, whole seconds.
pub fn watchdog_command(work: &Path, points: &[PathBuf], limit: Duration) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", WATCHDOG, "sh"])
        .arg(work)
        .arg(limit.as_secs().to_string())
        .args(points);
    cmd
}

fn start_watchdog(work: &Path) -> Result<(Child, ChildStdin), String> {
    let points: Vec<PathBuf> = IMAGES
        .iter()
        .map(|(_, _, _, point)| work.join(point))
        .collect();
    let mut child = watchdog_command(work, &points, DETACH_LIMIT)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Cannot start the images' watchdog: {e}"))?;
    let stdin = child.stdin.take().expect("stdin was asked for");
    Ok((child, stdin))
}

/// The test each binary with `#[ignore]`d image tests has: the `vars`
/// images mounted, every ignored test of the binary run on them, and the
/// images detached. A failure to mount, to pass or to detach fails it: it
/// never passes with nothing checked.
pub fn ignored_tests_pass_on_images(vars: &[&str], must_run: &[&str]) {
    let images = Images::mount(vars).unwrap_or_else(|e| panic!("{e}"));
    let tests = images.run_ignored_tests(must_run);
    let detached = images.detach();
    match (tests, detached) {
        (Ok(count), Ok(())) => println!("{count} tests passed on real mounts"),
        (Ok(_), Err(e)) | (Err(e), Ok(())) => panic!("{e}"),
        (Err(tests), Err(detach)) => panic!("{tests}\n{detach}"),
    }
}

/// What every image test says when a variable names what is no image.
pub const NO_IMAGE: &str = "is not a test image's mount";

/// `path`, resolved, if it is the mount of an image these tests or `scripts/test_exfat.sh` made:
/// a mount point of the harness's name, beside its image file, in a folder of the harness's name.
/// Image tests delete what they find there, as the images are theirs, so a variable that names a
/// real card or drive must stop them first. No shell-out: this runs before every test's first
/// file. The Linux run in the README names its folder and images to fit.
pub fn test_image_mount(path: &Path) -> Result<PathBuf, String> {
    let no = |why: &str| format!("{} {NO_IMAGE}: {why}", path.display());
    let path = fs::canonicalize(path).map_err(|e| no(&e.to_string()))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let Some((_, volume, ..)) = IMAGES.iter().find(|(_, _, _, point)| *point == name) else {
        return Err(no("its name is not one the harness gives"));
    };
    let Some(work) = path.parent() else {
        return Err(no("it has no parent"));
    };
    let work_name = work.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !WORK_FOLDERS.iter().any(|start| work_name.starts_with(start)) {
        return Err(no("its folder's name is not one the harness gives"));
    }
    let has_image = |file: String| work.join(file).is_file();
    if !has_image(format!("{volume}.dmg")) && !has_image(format!("{name}.img")) {
        return Err(no("no image file stands beside it"));
    }
    let device = |at: &Path| fs::metadata(at).map(|m| m.dev()).map_err(|e| no(&e.to_string()));
    if device(&path)? == device(work)? {
        return Err(no("nothing is mounted there"));
    }
    Ok(path)
}

/// Whether a test may remove `path`: inside a test image, not the mount itself, or inside this
/// test binary's own build folder.
pub fn may_delete(path: &Path) -> bool {
    let build = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|dir| fs::canonicalize(dir).ok());
    path.ancestors().skip(1).any(|above| {
        test_image_mount(above).is_ok() || fs::canonicalize(above).ok() == build
    })
}

/// Called before a helper deletes what a test gave it.
pub fn assert_may_delete(path: &Path) {
    assert!(
        may_delete(path),
        "refusing to delete {}: it {NO_IMAGE}, nor in one, nor in this test's build folder",
        path.display()
    );
}

/// Every file under `dir`, with its bytes, and every folder.
fn all_under(dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if fs::symlink_metadata(&path).unwrap().is_dir() {
            out.insert(path.clone(), None);
            all_under(&path, out);
        } else {
            out.insert(path.clone(), Some(fs::read(&path).unwrap()));
        }
    }
}

/// The test each binary with image tests has beside `ignored_tests_pass_on_images`: with `vars`
/// set to folders that hold photos and are no images, every ignored test of the binary fails,
/// saying so, and not a file is gone. Once with plain folders, and once with folders named as
/// the harness names mounts, beside an image file, with nothing mounted there.
pub fn ignored_tests_delete_nothing_outside_images(vars: &[&str]) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().unwrap();
    let list = Command::new(&exe).args(["--list", "--ignored"]).output().unwrap();
    let list = String::from_utf8(list.stdout).unwrap();
    let count = list.lines().filter(|l| l.ends_with(": test")).count();
    assert!(count > 0, "no ignored test to run: {list}");

    /// Removed on a failure too, as it holds nothing of anyone's.
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    for folder in ["plain", "photosync-images-0-0"] {
        let scratch = std::env::temp_dir().join(format!(
            "photosync-no-image-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&scratch).unwrap();
        let scratch = Scratch(fs::canonicalize(scratch).unwrap());
        let temp = scratch.0.join("temp");
        let work = scratch.0.join(folder);
        fs::create_dir_all(&temp).unwrap();
        let mut env = vec![("TMPDIR", temp.clone())];
        for var in vars {
            let (_, volume, _, point) = IMAGES.iter().find(|(v, ..)| v == var).unwrap();
            let there = work.join(point);
            fs::create_dir_all(&work).unwrap();
            fs::write(work.join(format!("{volume}.dmg")), b"").unwrap();
            for held in ["DCIM/100_FUJI", "Photos/2020-01-01/RAW"] {
                fs::create_dir_all(there.join(held)).unwrap();
                fs::write(there.join(held).join("DSCF0001.RAF"), b"a real photo").unwrap();
            }
            env.push((*var, there));
        }
        let mut before = BTreeMap::new();
        all_under(&work, &mut before);

        let log = scratch.0.join("log");
        fs::create_dir(&log).unwrap();
        let running = start(
            Command::new(&exe)
                .args(["--ignored", "--nocapture"])
                .envs(env.iter().cloned()),
            &log,
        )
        .unwrap();
        let (status, output) = running.finish(TESTS_LIMIT, "the tests on no image").unwrap();

        let mut after = BTreeMap::new();
        all_under(&work, &mut after);
        assert!(after == before, "{folder}: a test deleted or changed files:\n{output}");
        let want = format!("test result: FAILED. 0 passed; {count} failed; 0 ignored");
        assert!(!status.success() && output.contains(&want), "{folder}: wanted \"{want}\":\n{output}");
        let said = output.matches(NO_IMAGE).count();
        assert!(said >= count, "{folder}: {said} of {count} say why:\n{output}");
    }
}
