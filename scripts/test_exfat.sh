#!/usr/bin/env bash
# Runs the photosync-core and photosync tests with TMPDIR on a fresh exFAT
# disk image, so every test file lives on exFAT (macOS only: needs hdiutil).
# A second small image with a DCIM folder at its top stands in for a card:
# its mount path goes to the tests in PHOTOSYNC_TEST_CARD, and the INV-10
# end-to-end test plans a library on it with the real volume lookup. A third
# small image stands in for the external drive (PHOTOSYNC_TEST_LIBRARY): the
# binary's end-to-end test syncs the card to it, on three devices as strict
# INV-10 requires. Two more small images are two cards for the twin tests
# (PHOTOSYNC_TEST_TWIN_A, PHOTOSYNC_TEST_TWIN_B): a twin must be on another
# device than the file it stands in for (DEC-038). One HFS+ image runs the
# name tests again (`hfs_tests`): HFS+, like exFAT, lists a name decomposed.
# All images are made in a
# temp dir outside the repo and detached at the end, even on failure; if a
# detach fails, the script says so and removes nothing.
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
    a_lookup_the_library_refuses_is_an_error                    # copy_engine
    inv1_scan_with_failures_leaves_source_manifest_unchanged    # source_scan
    inv6_holds_per_source_over_every_kind_of_outcome            # report
    unreadable_scan_entries_are_listed_and_only_folders_have_unknown_contents  # report
    duplicate_of_an_original_that_failed_to_read_is_failed_not_skipped        # report
    a_readable_twin_of_an_original_that_failed_to_read_is_copied_by_the_fallback  # report
    a_twin_the_fallback_copied_is_the_original_for_later_cards                # session
    volume::tests::read_only_volume_is_not_a_destination                      # unit
    volume::tests::volume_whose_dcim_cannot_be_checked_is_a_failure_not_a_destination
    plan::tests::inv10_library_whose_volume_top_cannot_be_checked_is_refused   # unit
)

# Run on HFS+ too, by exact name, and counted: a misspelt name would run
# nothing and pass.
hfs_tests=(
    accented_name_rerun_on_a_later_date_lands_in_the_first_folder
    name_in_another_spelling_reuses_a_folder_only_if_the_filesystem_resolves_it
    only_the_folder_the_name_resolves_to_is_reused
    name_too_long_for_a_lookup_reuses_nothing_and_is_not_an_error
)

if [[ "$(uname)" != "Darwin" ]]; then
    echo "test_exfat.sh: macOS only (needs hdiutil)" >&2
    exit 1
fi
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

# Resolved, so the path matches what `mount` prints (/var is /private/var).
work="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/photosync-exfat.XXXXXX")" && pwd -P)"
mnt="$work/mnt"
card="$work/card"
library="$work/lib"
twin_a="$work/twin_a"
twin_b="$work/twin_b"
hfs="$work/hfs"
# Whole-disk devices of attached images whose file is under $1. A failing
# `hdiutil info` prints a placeholder, so the caller counts it as left.
attached_under() {
    local info
    if ! info=$(hdiutil info); then
        echo "hdiutil-info-failed"
        return
    fi
    awk -v dir="$1/" '
        /^image-path/ { sub(/^image-path *: */, ""); mine = (index($0, dir) == 1); next }
        mine && /^\/dev\/disk[0-9]+[ \t]/ { print $1; mine = 0 }
    ' <<<"$info"
}

cleanup() {
    local status=$?
    # Every mount point is detached and checked, attached or not: one
    # attach may have mounted before it failed.
    local point mounts left=()
    for point in "$hfs" "$twin_b" "$twin_a" "$library" "$card" "$mnt"; do
        if mounts=$(mount) && [[ $mounts == *" on $point "* ]]; then
            hdiutil detach "$point" -quiet || hdiutil detach "$point" -force -quiet || true
        fi
        # Checked again, not trusted to the detach: `rm -rf` through a live
        # mount would delete inside the image, and the image file under it.
        # Captured, not piped into `grep -q` (an early exit can SIGPIPE
        # `mount`), and a failing `mount` counts as still mounted, so the
        # check fails safe.
        if ! mounts=$(mount) || [[ $mounts == *" on $point "* ]]; then
            left+=("$point")
        fi
    done
    # An image can be attached without being mounted (an attach killed
    # midway); no mount point shows it, so detach by image path as well.
    local dev
    for dev in $(attached_under "$work"); do
        hdiutil detach "$dev" -quiet || hdiutil detach "$dev" -force -quiet || true
    done
    for dev in $(attached_under "$work"); do
        left+=("$dev")
    done
    if (( ${#left[@]} )); then
        echo "test_exfat.sh: FAILED to detach ${left[*]}; left them and $work in place" >&2
        echo "test_exfat.sh: detach by hand: hdiutil detach <each> -force; rm -rf '$work'" >&2
        exit $(( status ? status : 1 ))
    fi
    rm -rf "$work"
}
trap cleanup EXIT

# Makes an exFAT image named `$1` of size `$2` and mounts it at `$3`.
attach_exfat() {
    local name="$1" size="$2" point="$3"
    mkdir "$point"
    hdiutil create -size "$size" -fs ExFAT -volname "$name" "$work/$name.dmg" -ov -quiet
    hdiutil attach "$work/$name.dmg" -nobrowse -mountpoint "$point" -quiet
    if ! mount | grep -F " on $point " | grep -q "(exfat"; then
        echo "test_exfat.sh: $point is not mounted as exFAT" >&2
        exit 1
    fi
}

# Makes an HFS+ image named `$1` of size `$2` and mounts it at `$3`.
attach_hfs() {
    local name="$1" size="$2" point="$3"
    mkdir "$point"
    hdiutil create -size "$size" -fs HFS+ -volname "$name" "$work/$name.dmg" -ov -quiet
    hdiutil attach "$work/$name.dmg" -nobrowse -mountpoint "$point" -quiet
    if ! mount | grep -F " on $point " | grep -q "(hfs"; then
        echo "test_exfat.sh: $point is not mounted as HFS+" >&2
        exit 1
    fi
}

attach_exfat PSEXFAT 64m "$mnt"
echo "exFAT image mounted at $mnt"
attach_exfat PSCARD 16m "$card"
mkdir "$card/DCIM"
echo "card image (DCIM at its top) mounted at $card"
attach_exfat PSLIB 16m "$library"
echo "library drive image mounted at $library"
attach_exfat PSTWINA 16m "$twin_a"
attach_exfat PSTWINB 16m "$twin_b"
echo "twin card images mounted at $twin_a and $twin_b"
attach_hfs PSHFS 64m "$hfs"
echo "HFS+ image mounted at $hfs"

failed=0
args=()
for name in ${skip[@]+"${skip[@]}"}; do
    args+=(--skip "$name")
done
TMPDIR="$mnt" PHOTOSYNC_TEST_CARD="$card" PHOTOSYNC_TEST_LIBRARY="$library" \
    PHOTOSYNC_TEST_TWIN_A="$twin_a" PHOTOSYNC_TEST_TWIN_B="$twin_b" \
    cargo test --manifest-path "$repo/Cargo.toml" -p photosync-core -p photosync \
    --no-fail-fast -- --exact ${args[@]+"${args[@]}"} || failed=$?

# Run even if the above failed, so a failure on one format hides no other.
if ! TMPDIR="$hfs" cargo test --manifest-path "$repo/Cargo.toml" -p photosync-core \
    --test copy_engine --no-fail-fast -- --exact "${hfs_tests[@]}" | tee "$work/hfs.log"; then
    failed=1
elif ! grep -q "test result: ok. ${#hfs_tests[@]} passed" "$work/hfs.log"; then
    echo "test_exfat.sh: expected ${#hfs_tests[@]} tests to pass on HFS+" >&2
    failed=1
fi
exit "$failed"
