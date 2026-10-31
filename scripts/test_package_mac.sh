#!/usr/bin/env bash
# Checks what scripts/package_mac.sh made. Run by hand: at its end it opens the real
# `.app` twice, and each time a window shows on the screen for about a second, so this is not in the gate
# (S058, DEC-072 (1)). Keep the mouse off that window: a hovered button changes the picture.
set -euo pipefail

if [[ "$(uname)" != Darwin ]]; then
    echo "test_package_mac.sh: macOS only" >&2
    exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="$repo/target/package"
name="phoosync pro 2"
app="$package/$name.app"
zip="$package/$name.zip"
binary="$app/Contents/MacOS/photosync-gui"
plist="$app/Contents/Info.plist"
smoke="$package/smoke.png"
errors="$package/smoke.stderr"
bare_smoke="$package/smoke_no_arguments.png"
limit=60

if [[ ! -x "$binary" || ! -f "$plist" || ! -f "$zip" ]]; then
    echo "test_package_mac.sh: no package in $package: run scripts/package_mac.sh" >&2
    exit 1
fi

unzipped=""
root=""
cleanup() {
    if [[ -n "$unzipped" ]]; then rm -rf "$unzipped"; fi
    # rmdir, not rm: the folder must still be empty, as the run looked in it.
    if [[ -n "$root" ]]; then rmdir "$root"; fi
}
trap cleanup EXIT

failed=0
ok() { echo "ok: $1"; }
bad() {
    echo "FAILED: $1" >&2
    failed=1
}

plutil -lint "$plist" >/dev/null && ok "plutil -lint" || bad "plutil -lint"

# Read from Cargo.toml itself, not as the package script reads it, so a wrong read there shows.
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$repo/Cargo.toml" | head -n 1)"
value_is() {
    local got
    got="$(plutil -extract "$1" raw -o - "$plist" 2>&1)" || got="(no $1: $got)"
    if [[ "$got" == "$2" ]]; then
        ok "$1 = $got"
    else
        bad "$1: wanted '$2', got '$got'"
    fi
}
value_is CFBundleExecutable photosync-gui
value_is CFBundleIdentifier local.photosync.gui
value_is CFBundleName "$name"
value_is CFBundleDisplayName "$name"
value_is CFBundlePackageType APPL
value_is CFBundleShortVersionString "$version"
value_is CFBundleVersion "$version"
value_is NSHighResolutionCapable true
value_is NSRemovableVolumesUsageDescription \
    "phoosync pro 2 reads photos from your camera card and copies them to the drive you pick."

if codesign --verify --strict "$app"; then
    ok "codesign --verify --strict"
else
    bad "codesign --verify --strict"
fi

# The first line is the file's own name. An empty list must not pass, so count them.
libraries="$(otool -L "$binary" | tail -n +2 | awk '{print $1}')"
outside="$(printf '%s\n' "$libraries" | grep -v -E '^(/System/|/usr/lib/)' || true)"
if [[ -z "$libraries" ]]; then
    bad "otool -L lists nothing"
elif [[ -n "$outside" ]]; then
    bad "otool -L lists outside /System and /usr/lib: $outside"
else
    ok "otool -L: $(printf '%s\n' "$libraries" | wc -l | tr -d ' ') libraries, all in /System and /usr/lib"
fi

# Every entry with its kind and mode, as `diff -r` alone misses a lost executable bit.
listing() {
    (cd "$1" && find . -exec stat -f '%HT %Lp %N' {} + | LC_ALL=C sort)
}
unzipped="$(mktemp -d "$package/unzipped.XXXXXX")"
ditto -x -k "$zip" "$unzipped"
if diff -r "$app" "$unzipped/$name.app" && diff <(listing "$app") <(listing "$unzipped/$name.app"); then
    ok "the zip unzips to the same files: $(listing "$app" | wc -l | tr -d ' ') entries"
else
    bad "the zip does not unzip to the same files"
fi
if codesign --verify --strict "$unzipped/$name.app"; then
    ok "codesign --verify --strict on the unzipped copy"
else
    bad "codesign --verify --strict on the unzipped copy"
fi

# No window for a package already known bad.
if ((failed)); then
    echo "test_package_mac.sh: failed, window not opened" >&2
    exit 1
fi

rm -f "$smoke" "$errors" "$bare_smoke"
# Under target/, and empty, so a mounted card changes nothing and no removable volume is touched.
root="$(mktemp -d "$package/smoke_root.XXXXXX")"
# The name is random, so this finds only the run begun here. The binary's name leaves out
# `open` itself, which carries the same arguments.
this_run="photosync-gui.*$(basename "$root")"

# Absolute paths, as `open` starts the app in `/`.
open -n --stderr "$errors" "$app" --args --search-root "$root" --smoke-png "$smoke" &
opener=$!

# Done when the process has gone, not when the file appears: the file is written in place,
# so one seen early can be cut short. $1: pgrep's pattern. $2: the picture. $3: the file of
# stderr, or empty. $4: `open`'s pid. $5: what to run once, on the pids, when first seen.
wait_for_the_app() {
    local running seen=0 started=$SECONDS
    while :; do
        running="$(pgrep -f "$1" || true)"
        if [[ -n "$running" ]]; then
            if ((!seen)); then $5 $running; fi
            seen=1
        elif [[ -f "$2" ]]; then
            break
        elif ((seen)); then
            echo "FAILED: the app ended and wrote no picture" >&2
            if [[ -n "$3" ]]; then cat "$3" >&2 || true; fi
            exit 1
        fi
        if ((SECONDS - started > limit)); then
            echo "FAILED: the window did not end in $limit s; stopping it" >&2
            if [[ -n "$running" ]]; then kill $running || true; fi
            kill "$4" 2>/dev/null || true
            if [[ -n "$3" ]]; then cat "$3" >&2 || true; fi
            exit 1
        fi
        sleep 0.1
    done
    if ! wait "$4"; then
        echo "FAILED: open said no" >&2
        exit 1
    fi
}
wait_for_the_app "$this_run" "$smoke" "$errors" "$opener" :
ok "the app ran from open and wrote $smoke"

# `open` hides the exit code, so a panic or a refusal can only be seen here.
if [[ -s "$errors" ]]; then
    echo "FAILED: the app wrote to stderr:" >&2
    cat "$errors" >&2
    exit 1
fi
ok "the app wrote nothing to stderr"

compare_with_headless() {
    local log="$package/smoke_compare.log"
    if ! (cd "$repo" && PHOTOSYNC_GUI_SMOKE_PNG="$1" \
        cargo test -p photosync-gui --test smoke_window -- --ignored) >"$log" 2>&1; then
        cat "$log" >&2
        echo "FAILED: the package's window against the headless picture: $1" >&2
        exit 1
    fi
    # Zero tests would also exit 0.
    if ! grep -q "test result: ok. 1 passed" "$log"; then
        cat "$log" >&2
        echo "FAILED: the comparison did not run" >&2
        exit 1
    fi
}
compare_with_headless "$smoke"
ok "the package's window against the headless picture: 0 pixels off"

# As near a Finder launch as a script comes: no arguments and no --stderr of its own, the
# environment is only the test seam's, so what the app is handed is what macOS adds (S068).
# Not Finder itself: a double click stays S060's, as Finder passes no seam.
bare_run="^$(printf '%s' "$binary" | sed 's/[][\.*^$+?(){}|]/\\&/g')( |\$)"
if pgrep -f "$bare_run" >/dev/null; then
    echo "FAILED: this package is already running; quit it first" >&2
    exit 1
fi
what_macos_gave=""
note_what_macos_gave() {
    local pid="$1"
    what_macos_gave="arguments: $(ps -o args= -p "$pid" || true); stdout and stderr: $(lsof -a -p "$pid" -d 1,2 -Fn 2>/dev/null | sed -n 's/^n//p' | tr '\n' ' ' || true)"
}
open -n --env "PHOTOSYNC_GUI_SEAM_SEARCH_ROOT=$root" --env "PHOTOSYNC_GUI_SEAM_SMOKE_PNG=$bare_smoke" "$app" &
opener=$!
wait_for_the_app "$bare_run" "$bare_smoke" "" "$opener" note_what_macos_gave
ok "the app ran from open with no arguments of its own and wrote $bare_smoke"
echo "seen from open, not from Finder: ${what_macos_gave:-the process was gone before it was looked at}"
compare_with_headless "$bare_smoke"
ok "that window against the headless picture: 0 pixels off"
echo "test_package_mac.sh: passed. Read $smoke: it is what the real .app drew."
