#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: scripts/cut-release.sh [OPTIONS]

Cut a touchstone release from main.

Options:
  --dry-run                 Preview the release without mutating git or GitHub
  --version VERSION         Release version, e.g. 0.14.2 (defaults to next patch)
  --notes-file PATH         Markdown release notes for gh release create
  --print-current-version   Print the Cargo.toml package version and exit
  --print-next-version      Print the inferred next patch version and exit
  -h, --help                Show this help

The real release path updates Cargo.toml, Cargo.lock, AGENTS.md, and CLAUDE.md,
runs validation, commits the version bump, pushes main, and creates a GitHub
release. The GitHub release event publishes the crate through CI.
USAGE
}

current_version() {
    sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n 1
}

next_patch_version() {
    local version="$1"
    if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
        echo "error: cannot infer next patch from non-SemVer version: $version" >&2
        exit 1
    fi

    echo "${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.$((BASH_REMATCH[3] + 1))"
}

validate_version() {
    local version="$1"
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "error: version must be SemVer without a leading v, e.g. 0.14.2" >&2
        exit 1
    fi
}

run() {
    if [[ "$dry_run" == true ]]; then
        printf '+'
        printf ' %q' "$@"
        printf '\n'
    else
        "$@"
    fi
}

require_clean_tree() {
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "error: working tree must be clean before cutting a release" >&2
        git status --short >&2
        exit 1
    fi
}

require_main_branch() {
    local branch
    branch="$(git branch --show-current)"
    if [[ "$branch" != "main" ]]; then
        echo "error: releases must be cut from main, current branch is $branch" >&2
        exit 1
    fi
}

require_synced_main() {
    run git fetch origin main --tags

    if [[ "$dry_run" == true ]]; then
        return
    fi

    local local_head origin_head
    local_head="$(git rev-parse HEAD)"
    origin_head="$(git rev-parse origin/main)"
    if [[ "$local_head" != "$origin_head" ]]; then
        echo "error: local main must match origin/main before release" >&2
        echo "local:  $local_head" >&2
        echo "origin: $origin_head" >&2
        exit 1
    fi
}

require_absent_tag() {
    local tag="$1"

    if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
        echo "error: local tag already exists: $tag" >&2
        exit 1
    fi

    if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
        echo "error: remote tag already exists: $tag" >&2
        exit 1
    fi
}

update_version_files() {
    local old_version="$1"
    local new_version="$2"

    perl -0pi -e "s/(^version = \")\Q$old_version\E(\")/\${1}$new_version\${2}/m" Cargo.toml
    perl -0pi -e "s/(name = \"touchstone\"\\nversion = \")\Q$old_version\E(\")/\${1}$new_version\${2}/" Cargo.lock
    perl -0pi -e "s/v\Q$old_version\E/v$new_version/g" AGENTS.md CLAUDE.md
}

dry_run=false
version=""
notes_file=""
print_current=false
print_next=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            dry_run=true
            shift
            ;;
        --version)
            version="${2:-}"
            shift 2
            ;;
        --notes-file)
            notes_file="${2:-}"
            shift 2
            ;;
        --print-current-version)
            print_current=true
            shift
            ;;
        --print-next-version)
            print_next=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

current="$(current_version)"
if [[ -z "$current" ]]; then
    echo "error: could not read package version from Cargo.toml" >&2
    exit 1
fi

if [[ "$print_current" == true ]]; then
    echo "$current"
    exit 0
fi

if [[ -z "$version" ]]; then
    version="$(next_patch_version "$current")"
fi
validate_version "$version"

if [[ "$print_next" == true ]]; then
    echo "$version"
    exit 0
fi

if [[ "$version" == "$current" ]]; then
    echo "error: release version must differ from current version $current" >&2
    exit 1
fi

tag="v$version"

if [[ -z "$notes_file" ]]; then
    echo "error: --notes-file is required for release creation" >&2
    exit 1
fi

if [[ ! -f "$notes_file" ]]; then
    echo "error: notes file does not exist: $notes_file" >&2
    exit 1
fi

require_main_branch
require_clean_tree
require_synced_main
require_absent_tag "$tag"

echo "Preparing touchstone $tag from $current"
echo "Release notes: $notes_file"

if [[ "$dry_run" == true ]]; then
    echo
    echo "Dry run only; no files, commits, tags, pushes, or releases will be created."
    echo "Planned version file updates:"
    echo "+ Cargo.toml: $current -> $version"
    echo "+ Cargo.lock: $current -> $version"
    echo "+ AGENTS.md and CLAUDE.md release target: v$current -> v$version"
    echo
fi

if [[ "$dry_run" == false ]]; then
    update_version_files "$current" "$version"
fi

run just check
run cargo package
run git add Cargo.toml Cargo.lock AGENTS.md CLAUDE.md
run git commit -m "chore: prepare $tag release"
run git push origin main
run gh release create "$tag" \
    --repo iancleary/touchstone \
    --target "$(git rev-parse HEAD)" \
    --title "$tag" \
    --notes-file "$notes_file"

if [[ "$dry_run" == true ]]; then
    echo "Dry run complete; release would be created at https://github.com/iancleary/touchstone/releases/tag/$tag"
else
    echo "Release created: https://github.com/iancleary/touchstone/releases/tag/$tag"
fi
