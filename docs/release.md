# Release Process

Touchstone uses SemVer versions from `Cargo.toml` and `Cargo.lock`. Git tags and
GitHub releases use the same version with a leading `v`, for example `v0.14.1`.

The checked-in release entrypoint is:

```bash
just cut-release --notes-file /tmp/touchstone-v0.14.2-release.md
```

By default the runner infers the next patch version from `Cargo.toml`. Use
`--version X.Y.Z` when cutting a minor or otherwise non-patch release.

## Required State

- Run from `main`.
- The working tree must be clean.
- Local `main` must match `origin/main`.
- The target local and remote tag must not already exist.
- Release notes must be written to a local markdown file and passed with
  `--notes-file`.

## What The Runner Does

For a real release, `scripts/cut-release.sh`:

1. Fetches `origin/main` and tags.
2. Updates `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, and `CLAUDE.md`.
3. Runs `just check`.
4. Runs `cargo package`.
5. Commits the version bump.
6. Pushes `main`.
7. Creates the GitHub release with `gh release create`.

The GitHub release event runs CI and publishes the crate to crates.io through
`.github/workflows/ci.yml`.

## Preview And Version Queries

Preview the release without mutating files, committing, pushing, tagging, or
creating a GitHub release:

```bash
just cut-release --dry-run --notes-file /tmp/touchstone-v0.14.2-release.md
```

Print the current version:

```bash
just cut-release --print-current-version
```

Print the next inferred patch version:

```bash
just cut-release --print-next-version
```

## Release Notes

Use a file-backed notes workflow for release notes. Keep notes concise and
focused on user-facing changes, validation, and any compatibility notes.

Example:

```markdown
## Highlights

- Add ...

## Validation

- `just check`
- `cargo package`
```
