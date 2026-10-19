#!/usr/bin/env bash
# Checks each way a host can link SQLite: builds the linkage probe in each, and
# checks that the probe reports the library that mode should link. Linux only.
#
# Usage: check-linkage.sh [mode...]
#   Modes: api, bundled, system, static, dynamic. With none, runs every mode,
#   skipping `system` when `pkg-config` finds no SQLite. A mode named
#   explicitly fails rather than being skipped.
#
# `api` lints `frostlang-sqlite` without `bundled`, which disables API that
# `cargo test` and the workspace's lints otherwise always see.
#
# `static` and `dynamic` compile the amalgamation that `libsqlite3-sys` ships,
# each tagged with a compile-time option no other mode's library sets, so the
# probe can tell which library it got.
#
# Needs `cargo` with Clippy, `jq`, a C compiler as `cc`, and `ar`.

set -euo pipefail

if [[ $(uname -s) != Linux ]]; then
    echo "check-linkage.sh runs only on Linux" >&2
    exit 2
fi

probe_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null && pwd)
manifest="$probe_dir/Cargo.toml"
# Every mode shares one target directory, so only `libsqlite3-sys` and its
# dependents rebuild between modes. It is apart from the workspace's, which
# would otherwise rebuild them on the next ordinary build.
target_dir="$(cargo metadata --manifest-path "$manifest" --format-version 1 --no-deps |
    jq -r .target_directory)/sqlite-linkage"
amalgamation_dir="$(dirname "$(cargo metadata --manifest-path "$manifest" --format-version 1 |
    jq -r '.packages[] | select(.name == "libsqlite3-sys") | .manifest_path')")/sqlite3"
amalgamation_version=$(sed -n 's/^#define SQLITE_VERSION *"\(.*\)"$/\1/p' "$amalgamation_dir/sqlite3.h")

# The option tagging each custom-built library.
static_tag=MAX_ATTACHED=7
dynamic_tag=MAX_ATTACHED=8

# Every variable the build reads to choose a library, unset for each mode so
# the caller's environment cannot change what it links.
linkage_vars=(
    SQLITE3_LIB_DIR SQLITE3_INCLUDE_DIR SQLITE3_STATIC SQLITE3_NO_PKG_CONFIG
    LIBSQLITE3_SYS_USE_PKG_CONFIG LIBSQLITE3_FLAGS
)

# Run a command without any of `linkage_vars`.
# Usage: clean_env [VAR=value...] command [args...]
clean_env() {
    env "${linkage_vars[@]/#/-u}" "$@"
}

explicit=$(($# > 0))
failures=()
skipped=()

fail() {
    echo "FAILED: $2"
    failures+=("$1")
}

skip() {
    if ((explicit)); then
        fail "$1" "$2, but the mode was requested"
    else
        echo "SKIPPED: $2"
        skipped+=("$1")
    fi
}

# Run the probe built with the environment and Cargo arguments given, and check
# that it reports `expected_version`, `expected_option` if not empty, and no
# other mode's tag.
# Usage: check <mode> <expected_version> <expected_option> [VAR=value...] -- [cargo args...]
check() {
    local mode=$1 expected_version=$2 expected_option=$3
    shift 3
    local env=()
    while [[ $1 != -- ]]; do
        env+=("$1")
        shift
    done
    shift

    local report
    if ! report=$(clean_env ${env[@]+"${env[@]}"} cargo run --quiet --locked \
        --manifest-path "$manifest" --target-dir "$target_dir" "$@"); then
        fail "$mode" "the probe did not build or run"
        return
    fi

    local version problem=""
    version=$(sed -n 's/^version //p' <<<"$report")
    if [[ $version != "$expected_version" ]]; then
        problem="expected SQLite $expected_version, but the probe reports $version"
    elif [[ -n $expected_option ]] && ! grep -qx "option $expected_option" <<<"$report"; then
        problem="the library lacks compile option $expected_option"
    else
        local tag
        for tag in "$static_tag" "$dynamic_tag"; do
            if [[ $tag != "$expected_option" ]] && grep -qx "option $tag" <<<"$report"; then
                problem="the library has compile option $tag, which tags another mode's library"
            fi
        done
    fi
    if [[ -n $problem ]]; then
        fail "$mode" "$problem"
        echo "The probe reported:"
        echo "$report"
        return
    fi
    echo "ok: SQLite $version${expected_option:+, with $expected_option}"
}

# Compile the amalgamation into `dir` as `kind` (static or dynamic), with
# `option` (an `SQLITE_` compile-time option) set.
build_library() {
    local dir=$1 kind=$2 option=$3
    mkdir -p "$dir" &&
        command rm -f "$dir"/sqlite3.o "$dir"/libsqlite3.* || return
    case $kind in
        static)
            cc -O2 -fPIC -D"$option" -c "$amalgamation_dir/sqlite3.c" -o "$dir/sqlite3.o" &&
                ar rcs "$dir/libsqlite3.a" "$dir/sqlite3.o"
            ;;
        dynamic)
            cc -O2 -fPIC -shared -D"$option" "$amalgamation_dir/sqlite3.c" \
                -o "$dir/libsqlite3.so" -lpthread -lm
            ;;
    esac
}

# Check a library custom-built as `kind`, tagged with `tag`.
check_custom() {
    local kind=$1 tag=$2
    # Named for its contents, since Cargo does not notice a static library
    # replaced in place.
    local dir="$target_dir/custom-$kind-$amalgamation_version-${tag//=/-}"
    if ! build_library "$dir" "$kind" "SQLITE_$tag"; then
        fail "$kind" "the library did not build"
        return
    fi
    local env=(SQLITE3_LIB_DIR="$dir" SQLITE3_NO_PKG_CONFIG=1)
    case $kind in
        static) env+=(SQLITE3_STATIC=1) ;;
        dynamic) env+=(LD_LIBRARY_PATH="$dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}") ;;
    esac
    check "$kind" "$amalgamation_version" "$tag" "${env[@]}" --
}

check_api() {
    if clean_env cargo clippy --quiet --locked --manifest-path "$manifest" \
        --target-dir "$target_dir" -p frostlang-sqlite --lib -- -D warnings; then
        echo "ok: lints clean without bundled"
    else
        fail api "the library did not lint clean without bundled"
    fi
}

check_bundled() {
    # `libsqlite3-sys` sets this when bundling; a default build does not.
    check bundled "$amalgamation_version" ENABLE_API_ARMOR -- --features bundled
}

check_system() {
    if ! command -v pkg-config >/dev/null; then
        skip system "pkg-config is not installed"
    elif ! clean_env pkg-config --exists sqlite3; then
        skip system "pkg-config finds no SQLite"
    else
        check system "$(clean_env pkg-config --modversion sqlite3)" "" --
    fi
}

check_static() {
    check_custom static "$static_tag"
}

check_dynamic() {
    check_custom dynamic "$dynamic_tag"
}

modes=("$@")
if ((${#modes[@]} == 0)); then
    modes=(api bundled system static dynamic)
fi
for mode in "${modes[@]}"; do
    case $mode in
        api | bundled | system | static | dynamic) ;;
        *)
            echo "unknown mode: $mode" >&2
            exit 2
            ;;
    esac
done
for mode in "${modes[@]}"; do
    echo "== $mode"
    "check_$mode"
done

echo
if ((${#skipped[@]} > 0)); then
    echo "skipped: ${skipped[*]}"
fi
if ((${#failures[@]} > 0)); then
    echo "failed: ${failures[*]}"
    exit 1
fi
echo "every mode run passed"
