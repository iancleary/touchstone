---
name: verify-rf-numerics
description: Verify Touchstone numerical solves and common-real reference changes against analytic circuits, end-to-end metadata workflows, and bounded release experiments. Use when maintaining these implemented numerical APIs.
---

# Verify numerical network changes

## Prerequisites and scope

Follow repository `AGENTS.md`, inspect Git status, and preserve unrelated work.
Read the relevant implemented contract: `docs/numerical-solves.md` for S/Y/Z
solves or `docs/renormalization.md` for common-real reference changes. Existing
Cargo dependencies/toolchain must be available; do not install anything or
publish artifacts implicitly. This procedure does not authorize broader wave
conventions, tolerance changes, Git writes, or release execution.

## Sequence

1. Trace the public network/matrix API into the existing private solver. Preserve
   right-solve order and ordinary (not conjugate) transpose semantics. Do not
   introduce a second solver to hide a difficult edge case.
2. Identify an independent physical or rational oracle before editing: use
   one-port load equations or the T-resistor circuit documented in the reference
   change guide. A roundtrip or two paths sharing the solver is only a cross-check.
3. Run the focused existing public workflow:
   - `cargo test --offline --test numerical_solves`
   - `cargo test --offline --test renormalization`
4. Run the corresponding release experiment:
   - `cargo run --offline --release --example numerical_solves`
   - `cargo run --offline --release --example renormalization`
   Distinguish analytic accuracy checks from synthetic cross-checks and single
   elapsed timings. Do not present unequal-work totals as controlled speedups.
5. Check reference and data invariants: non-S sources, per-port semantics,
   ignored mixed-mode warnings (including public case/hyphen/whitespace edits),
   finite positive references, source grids/ranks, identity, extreme ratios, exact
   singularities, and the absolute pivot cutoff. Renormalization must reject
   unsupported source formats/units before identity: only exact RI/MA/DB and
   Hz/kHz/MHz/GHz are accepted. THz is writer-only and not a supported parser
   roundtrip; do not claim it as a passed renormalization workflow.
   Verify failure leaves source data unchanged. For metadata changes, exercise
   transform → serialize → parse as well as in-memory warnings/caches.
6. Run the repository gates without weakening tests or tolerance:
   `cargo fmt --all -- --check`,
   `cargo clippy --offline --all-targets --all-features -- -D warnings`,
   `cargo test --offline --all-features -- --test-threads=1`,
   `RUSTDOCFLAGS='-D warnings' cargo doc --offline --all-features --no-deps`,
   and `git diff --check`.

## Failure interpretation and recovery boundaries

- A pivot below `1e-12` is a scale-dependent policy failure, not a condition
  estimate. A small residual or finite answer is not a forward-error bound.
- Extreme positive references can have an overflowing sum or an unrepresentable
  ratio transform. Keep explicit rejection distinct from denominator singularity;
  do not silently clamp `g`, regularize, or promise all-f64-range support.
- Parser warnings survive a network clone/transform, but ignored source keywords
  are not serialized. Newly parsed output need not reproduce original warnings.
- Missing offline dependencies block that verification; do not replace the real
  path with mocks or install dependencies without authorization.
- Correct the narrow owning module and rerun the affected experiment plus gates.
  Preserve the user's original network; rollback is discarding only authorized
  changes after review, not destructive reset commands.

Report exact commands/results, unresolved numerical limitations, and the
remaining parent-review/package gate. Full packaging requires the repository's
clean-checkout workflow; do not commit merely to satisfy that gate without
permission. Deeper decisions, source links, and known limits live in the two
maintained documentation pages above, not in duplicated skill-owned code.
