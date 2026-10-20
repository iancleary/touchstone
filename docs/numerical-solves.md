# Complex solves for S/Y/Z conversions

## Mental model and entry points

A conversion is a matrix equation, not a request to construct an inverse. The
four public conversions in [`src/lib.rs`](../src/lib.rs) now use the private
[`ComplexLu`](../src/numerics.rs) partial-pivoting factorization:

| Conversion | Solve `X D = N` | Final result |
| --- | --- | --- |
| S → Y | `D = I + S`, `N = I - S` | `X / z0` |
| S → Z | `D = I - S`, `N = I + S` | `z0 X` |
| Y → S | `D = I + z0 Y`, `N = I - z0 Y` | `X` |
| Z → S | `D = Z + z0 I`, `N = Z - z0 I` | `X` |

The implementation solves `D^T X^T = N^T`. This is the **ordinary transpose**,
not a complex-conjugate transpose. Reversing multiplication order is not a
valid general right solve. These particular numerators and denominators commute
in exact arithmetic, so conversion tests alone cannot protect the reusable
primitive's order. A focused private test uses noncommuting complex matrices,
known left and right solutions, multiple RHS columns, and row swaps. The
public-API tests use independent rational and circuit references.

`ComplexLu::factor` records the row swaps for `P A = L U`; `solve` applies them to
all RHS columns, then performs forward and backward substitution. Crate-private
callers can reuse a factorization and supply rectangular RHS matrices (rank rows,
equal nonzero column counts). The right-solve helper currently accepts square
matrices. These are building blocks for later renormalization and connections,
not new implementations of those features. Public matrix storage is unchanged.
No production conversion forms an inverse; the historical Gauss–Jordan algorithm
survives only in the comparison example.

## Opt-in diagnostics

Existing methods keep their signatures. Four additive methods return
`Result<(Matrix, SolveDiagnostics), TouchstoneError>`:

- `SMatrix::to_y_matrix_with_diagnostics(z0)`
- `SMatrix::to_z_matrix_with_diagnostics(z0)`
- `ParameterMatrix::to_s_matrix_from_y_with_diagnostics(z0)`
- `ParameterMatrix::to_s_matrix_from_z_with_diagnostics(z0)`

For example, a 75-ohm load at a 50-ohm reference has `S11 = 0.2`:

```rust
use touchstone::{Complex, SMatrix};
let s = SMatrix::try_new(vec![vec![Complex { re: 0.2, im: 0.0 }]])?;
let (z, report) = s.to_z_matrix_with_diagnostics(50.0)?;
assert!((z.get(1, 1)?.re - 75.0).abs() < 1e-12);
assert_eq!(report.reciprocal_condition_inf, 1.0);
# Ok::<(), touchstone::TouchstoneError>(())
```

This example is also executable in the `SolveDiagnostics` rustdoc.
The report describes `X D = N` **before** the final S→Y/Z scalar scaling:

- `reciprocal_condition_inf = 1 / (||D||∞ ||D^-1||∞)`, capped at one for
  floating-point rounding. The infinity norm is the largest row sum of complex
  moduli. Smaller values indicate sensitivity to perturbations. A scalar
  denominator always has condition number one regardless of its magnitude;
  conditioning is not the cutoff test or a measure of scalar amplification.
- `backward_error_inf = ||X D - N||∞ / (||X||∞ ||D||∞ + ||N||∞)`.
  This normwise scaled residual measures how well the computed result satisfies
  the equation. It is **not** componentwise LAPACK BERR, a certified bound, or a
  forward-error guarantee. The all-zero numerator/solution gives zero error.

Internally these are 1-norm calculations on the transposed equation. The inverse
norm is accumulated one identity-column solve at a time: O(n³) extra work, O(n)
auxiliary column storage, and no stored full inverse. Residual evaluation is also
O(n³). The ordinary path does neither calculation. Factorization and a dense
multiple-RHS solve are O(n³) with O(n²) matrix storage.

The residual calculation normalizes matrices before multiplication and balances
norm weights without forming a potentially overflowing norm product. A log-based
fallback handles extreme intermediate norm ratios. Norm sums and auxiliary solves
must still fit f64: diagnostic variants may return
`InvalidParameterMatrixValue` even if the ordinary conversion succeeds. They
never deliberately return NaN as a report. Underflow or cancellation can produce
zero rcond/residual; zero residual is not exactness. Neither diagnostics nor
finite-result validation certify the original measurement or final unit scaling.

## Failure policy and compatibility boundary

All inputs and returned conversion entries must remain finite; the common
reference must be positive and finite. Every LU pivot must have finite magnitude
**strictly greater than `1e-12`**, using the existing error type and tolerance.
This absolute policy is scale dependent, not an accuracy budget. Historical error
operation labels containing “matrix inversion” are retained for compatibility,
even though the implementation now solves instead.

The same cutoff does **not** imply identical acceptance: transposition changes
partial-pivot selection, and LU vs Gauss–Jordan changes rounding. Two explicit
example probes demonstrate both directions (no existing tests were weakened):

1. `D = 2e-12 * [[1,1],[2,1]]`, `z0 = 2e-12/8`: old Gauss–Jordan rejects its
   second pivot (`1e-12`); LU on `D^T` accepts both pivots (`2e-12`).
2. `D = [[1e-12,1],[2e-12,1]]`, `z0 = 1e-15`: old Gauss–Jordan accepts;
   LU on `D^T` rejects its second pivot (`1e-12`).

Both are exercised as Z→S with `Z = D - z0 I`. The second case is a remaining
compatibility risk for callers operating near the absolute threshold; parent
review should explicitly consider it. There is no hidden threshold relaxation,
pseudoinverse, regularization, equilibration, or iterative refinement. Exact
singular denominators, overflowed calculations, and nonfinite results still
error. Direct two-port cascade formulas/cutoffs and the ABCD contract are
unchanged; see [ABCD investigation](abcd-numerics.md) and
[cascade consistency](cascade-consistency.md).

## Experiment and decisions

Run without new dependencies:

```sh
cargo run --offline --release --example numerical_solves
cargo test --offline --test numerical_solves
```

The [example](../examples/numerical_solves.rs) compares the historical algorithm
with the actual public LU paths for all four conversions at full ranks 2, 8, and
32. Dense matrices are synthetic, nonreciprocal, complex rank-one perturbations
of a diagonal matrix, with analytic Sherman–Morrison references, not a shared
round-trip oracle. Further probes use an exact rational two-port reference at
scales `1e-13` through `1e300`, a threshold-order difference, and an
ill-conditioned diagonal S matrix. These are bounded workflow experiments, not
coverage of arbitrary real measurements, every resonance, or a universal RF
accuracy envelope.

Measured on arm64, rustc `1.99.0 (b940084d7 2026-09-28)`, release profile:

| Rank / path | GJ max absolute entry error | LU error | GJ time | LU time | LU + diagnostics total |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2 S→Z | 1.422e-14 | 1.422e-14 | 1.416 µs | 12.209 µs | 2.875 µs |
| 8 S→Z | 1.424e-14 | 2.845e-14 | 4.334 µs | 6.375 µs | 12.542 µs |
| 32 S→Z | 5.684e-14 | 5.684e-14 | 128.625 µs | 60.250 µs | 247.250 µs |
| 32 S→Y | 8.674e-18 | 1.041e-17 | 123.375 µs | 54.375 µs | 199.791 µs |
| 32 Y→S | 2.776e-16 | 1.943e-16 | 123.000 µs | 55.917 µs | 197.917 µs |
| 32 Z→S | 2.220e-16 | 1.388e-16 | 122.750 µs | 53.875 µs | 201.167 µs |

Times are single shots, including allocation, without warmup/statistical
controls. First-call overhead is visible at rank 2. Baseline omits public input
validation. These timings isolate report-enabled vs ordinary calls, but do not
establish a speedup or an exact incremental diagnostic cost. LU is not uniformly
more accurate: rank-8 Z error doubled while remaining around rounding scale.
Across all 12 regular cases LU backward errors were below `2.7e-16` and rcond
was about `0.50..0.85`. At scale `1e300`, Z→S absolute error was `3.33e-16`,
rcond `1/9`, and the rounded residual was zero despite nonzero solution error.
For `S = diag(1-1e-10,0)`, rcond was `1.0000000827403709e-10` while backward
error was about `1.3e-26`: a concrete reason not to equate residual with accuracy.

| Choice | Decision / evidence |
| --- | --- |
| Narrow private partial-pivot LU | Chosen: one dependency-free factorization, multiple RHS, explicit order; analytic reference results pass at all three ranks. |
| Optional pure-Rust backend | Deferred, not installed or benchmarked. It could supply mature estimators/refinement later, but adds feature/dependency maintenance for this bounded slice. No evidence here claims our solver outperforms such a backend. |
| Keep Gauss–Jordan inverse | Rejected for production: materializes an unnecessary inverse and does not provide a reusable factor/solve. Kept only as experimental baseline. |
| Pivot magnitude as rcond | Rejected: scale-dependent cutoff is not conditioning. |
| Identity-column inverse-norm calculation | Chosen only for opt-in reporting: easy to audit at bounded ranks, costly O(n³), not a cheap condition estimator. |
| Relative cutoff / equilibration / refinement | Deferred pending a distinct compatibility and accuracy study; not silently bundled into this migration. |

## Verification and self-review

[`tests/numerical_solves.rs`](../tests/numerical_solves.rs) covers exact rational
complex nonreciprocal references in all directions, a `75+j25` ohm circuit,
ill-conditioning, zero RHS, exact singularities, scale/cutoff boundaries, and
extreme finite inputs (including report-only norm overflow). References are exact
rational identities or analytic formulas, not an external arbitrary-precision
backend. The private noncommuting test protects reusable solve order and complex
transpose semantics that these conversion polynomials cannot distinguish.

Required checks were run offline with available dependencies:

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --offline --all-features --no-deps
cargo run --offline --release --example numerical_solves
```

All five commands passed. The all-features test run passed 306 tests (148 library,
130 integration, and 28 doctests), with zero failures; the release example's 12
regular accuracy assertions also passed. `git diff --check` passed. No existing
tests or tolerances were changed. The first Clippy run found three range-loop
style warnings in the new solver; these were fixed before the passing full run.

Full package validation is deferred until the parent commit because the packaging
gate requires a clean checkout. Self-review checked pivot permutation, ordinary
transpose, RHS order, norm identities, finite-report behavior, cheap-path work,
and preservation of cascade/ABCD code. Architect/arena/comment-review tooling was
unavailable; no independent review is claimed here. Parent review remains required.

Throughput checkpoint (single implementation owner, no internal fan-out):

- [x] **Blocking first steps:** clean worktree inspected; supplied baseline
  `8b58738` had `just check` green; contracts read and runnable experiment run
  before finalizing numerical/report choices.
- [x] **Independent workstreams:** n/a — solver, conversion migration, experiment,
  tests, and numerical contract are one code-coupled slice.
- [x] **Shared mutable state:** isolated worktree; one owner; no Git writes,
  dependency installations, or writes outside this worktree.
- [x] **Smallest safe decomposition:** one reusable factorization and four thin
  conversion wrappers; no nested delegation or source-data redesign.
