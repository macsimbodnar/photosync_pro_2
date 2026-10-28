#!/usr/bin/env bash
# Runs the photosync-core tests with TMPDIR on a fresh exFAT disk image, so
# every test file lives on exFAT (macOS only: needs hdiutil). The image is
# made in a temp dir outside the repo and is detached at the end, even on
# failure; if the detach fails, the script says so and removes nothing.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# exFAT has no Unix permission bits: chmod does nothing there, so these tests
# cannot make a file or folder unreadable (the `Mode` helper). They fail, or
# pass without testing anything. Skipped by exact name; every other test,
# each copy and rename test included, runs on exFAT.
skip=(
    read_error_on_a_source_file_fails_that_file_only            # copy_engine
    only_name_and_size_candidates_are_hashed                    # library_plan
    unreadable_source_file_fails_that_file_only                 # library_plan
    unreadable_library_file_is_a_library_failure_and_the_file_is_planned
    unreadable_library_folder_is_a_library_failure              # library_plan
    unreadable_subfolder_is_a_failure_and_the_rest_is_listed    # source_scan
    unreadable_files_are_failures_and_the_rest_is_listed        # source_scan
    unreadable_dcim_is_an_error                                 # source_scan
    unreadable_root_is_an_error                                 # source_scan
    inv1_scan_with_failures_leaves_source_manifest_unchanged    # source_scan
    inv6_holds_per_source_over_every_kind_of_outcome            # report
    unreadable_scan_entries_are_listed_and_only_folders_have_unknown_contents
)

if [[ "$(uname)" != "Darwin" ]]; then
    echo "test_exfat.sh: macOS only (needs hdiutil)" >&2
    exit 1
fi
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

# Resolved, so the path matches what `mount` prints (/var is /private/var).
work="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/photosync-exfat.XXXXXX")" && pwd -P)"
mnt="$work/mnt"
attached=0
cleanup() {
    local status=$?
    if (( attached )); then
        hdiutil detach "$mnt" -quiet || hdiutil detach "$mnt" -force -quiet || true
    fi
    # Checked, not trusted to `attached`: `rm -rf` through a live mount would
    # delete inside the image, and the image file under it. Captured, not
    # piped into `grep -q` (an early exit can SIGPIPE `mount`), and a failing
    # `mount` counts as still mounted, so the check fails safe.
    local mounts
    if ! mounts=$(mount) || [[ $mounts == *" on $mnt "* ]]; then
        echo "test_exfat.sh: FAILED to detach $mnt; left it and $work in place" >&2
        echo "test_exfat.sh: detach by hand: hdiutil detach '$mnt' -force; rm -rf '$work'" >&2
        exit $(( status ? status : 1 ))
    fi
    rm -rf "$work"
}
trap cleanup EXIT

mkdir "$mnt"
hdiutil create -size 64m -fs ExFAT -volname PSEXFAT "$work/exfat.dmg" -ov -quiet
hdiutil attach "$work/exfat.dmg" -nobrowse -mountpoint "$mnt" -quiet
attached=1
if ! mount | grep -F " on $mnt " | grep -q "(exfat"; then
    echo "test_exfat.sh: $mnt is not mounted as exFAT" >&2
    exit 1
fi
echo "exFAT image mounted at $mnt"

args=()
for name in ${skip[@]+"${skip[@]}"}; do
    args+=(--skip "$name")
done
TMPDIR="$mnt" cargo test --manifest-path "$repo/Cargo.toml" -p photosync-core --no-fail-fast -- --exact ${args[@]+"${args[@]}"}
