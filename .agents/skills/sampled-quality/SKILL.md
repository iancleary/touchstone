---
name: sampled-quality
description: Diagnose sampled passivity and matrix symmetry of single-ended one- or two-port Touchstone S data, or verify changes to the reporting workflow.
---

# Sampled quality workflow

Use this procedure for a measured/simulated `.s1p` or `.s2p` network expected to
be passive, not for repairing measurements or certifying broadband behavior.

## Prerequisites and boundaries

- Run from a touchstone checkout with its existing Rust toolchain and cached
  dependencies. Do not install tools, fetch data, or publish results implicitly.
- Read `docs/sampled-quality.md` in that checkout for the wave/reference model,
  tolerance interpretation, algorithm evidence, and troubleshooting.
- Keep source measurements and parser warnings intact. Never bypass validation
  by clearing warnings or relabeling mixed-mode or non-S metadata.

## Sequence

1. Check Git status and preserve existing changes. Read repository instructions.
2. Load measurements through `Network::new`, `from_str`, or `from_bytes`, not the
   synthetic-data builder. Inspect every parser warning before interpreting data.
3. Confirm single-ended S semantics, one or two ports, positive finite real
   references, finite RI values, and a nonempty increasing nonnegative Hz grid.
4. Choose an explicit finite nonnegative absolute tolerance appropriate to source
   precision; it is not a measurement uncertainty estimate.
5. Call `sampled_quality(SampledQualityOptions { tolerance })`. Inspect raw
   `sigma_max`, `excess_above_one`, and passivity classification. Inspect symmetry
   separately at `worst_symmetry`; it may occur at a different sample than
   `worst_passivity`. Sample indexes are zero-based.
6. For a maintained real-API demonstration and two-formula numerical comparison,
   run `cargo run --offline --example sampled_quality`. For an additional local
   two-port input, append `-- path/to/input.s2p`. The example only reads inputs
   and prints results; the user input is not assumed passive.
7. For implementation changes, run `cargo test --offline --test sampled_quality`,
   then the repository's required format, all-target lint, all-feature tests,
   and warning-denied rustdoc gates. Do not equate compilation with runtime proof.

## Evidence and recovery

The maintained example covers lossless through, attenuator, coherent gain,
complex unitary, zero/rank-one degeneracy, tiny/large values, near-equal singular
values, and a real fixture. See the documentation for observed formula differences
and the fixture's small excess above one. These are bounded experiments, not
certified numerical error bounds.

On invalid metadata/grids, unsupported mixed-mode declarations, non-finite input,
or unrepresentable results, report the contextual error. Correct source semantics
or calibration outside this procedure rather than mutating a network to suppress
an error. N-port SVD, complex references, automatic repair, broadband passivity,
causality, and universal reciprocity remain unsupported. No rollback is required
for reporting: it performs no writes. Any code changes remain ordinary local
work and require separate authorization for commits or publication.
