#!/usr/bin/env bash
# Checks from this Mac that the whole workspace, tests too, compiles for Linux (S059, DEC-070 (3)).
# `cargo check` only: nothing is linked and nothing runs, so this does not prove the app works
# there. S025 and S061 run it on Linux. It needs the network, and changes nothing of the user's:
# the toolchain, its Linux targets and the crates go to a throwaway folder (DEC-089). Run by hand,
# as it downloads all that each time, so it is not in `cargo test` or `scripts/test_exfat.sh`.
set -euo pipefail

if [[ "$(uname)" != Darwin ]]; then
    echo "check_linux_from_mac.sh: macOS only" >&2
    exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
targets=(aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu)
user_rustup="${RUSTUP_HOME:-$HOME/.rustup}"

# Read before RUSTUP_HOME changes, and in the repo, where an override elsewhere cannot
# speak: the check is for the compiler the user's builds use.
version="$(cd "$repo" && rustc --version | cut -d ' ' -f 2)"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "check_linux_from_mac.sh: not a stable release: rustc says '$version'" >&2
    exit 1
fi

# Path, modified time and size of each entry: what an install or an update would change.
fingerprint() {
    find "$user_rustup" -exec stat -f '%N %m %z' {} + | LC_ALL=C sort
}
before="$(fingerprint)"

work="$(mktemp -d "${TMPDIR:-/tmp}/photosync-linux-check.XXXXXX")"
cleanup() {
    local code=$?
    trap - EXIT
    if ! rm -rf "$work"; then
        echo "FAILED: could not remove $work" >&2
        code=1
    fi
    exit "$code"
}
trap cleanup EXIT
# Without these, a signal would end bash and skip the EXIT trap.
trap 'exit 130' INT
trap 'exit 143' TERM

export RUSTUP_HOME="$work/rustup"
export CARGO_HOME="$work/cargo"
export CARGO_TARGET_DIR="$work/target"

target_flags=()
for target in "${targets[@]}"; do target_flags+=(--target "$target"); done
rustup toolchain install "$version" --profile minimal --no-self-update "${target_flags[@]}"

# blake3 compiles C for its NEON code on aarch64, and there is no Linux C compiler here. Its
# `pure` feature leaves that C out for this check only: the app's build keeps it (DEC-089).
flags_for() {
    if [[ "$1" == aarch64-* ]]; then echo "--features blake3/pure"; fi
}

# Every target runs, so one run shows all there is to fix.
cd "$repo"
failed=0
for target in "${targets[@]}"; do
    # Word splitting wanted: no flags, or the two words of a flag.
    # shellcheck disable=SC2046
    set -- --locked --workspace --all-targets --target "$target" $(flags_for "$target")
    echo "== cargo check $*"
    if rustup run "$version" cargo check "$@"; then
        echo "ok: $target"
    else
        echo "FAILED: $target" >&2
        failed=1
    fi
done

if [[ "$before" != "$(fingerprint)" ]]; then
    echo "FAILED: $user_rustup changed during this run" >&2
    failed=1
fi

if [[ "$failed" != 0 ]]; then
    exit 1
fi
echo "ok: it compiles for ${targets[*]} with rustc $version. Nothing was linked or run."
