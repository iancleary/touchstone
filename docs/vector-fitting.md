# EXPERIMENTAL bounded vector fitting

This optional modeling subsystem is **not a general broadband fitting backend**.
Synthetic in-class accuracy is demonstrated; the real measured fixture attempted
below **fails suitability**. Keep ordinary RI interpolation as the default.
Stable poles do not imply passivity, and convergence does not imply accuracy.

## Data versus a model

A `Network` contains sampled complex S matrices, a frequency grid in Hz,
reference impedances, and parser warnings. `sample_at` interpolates those samples.
`vector_fitting::RationalNetworkModel` is a separate, immutable hypothesis:

```text
S(s) = D + sum_k R_k / (s - p_k),  s = j 2π f
```

Every entry in the **full** one-port or two-port matrix has its own real D and
residue coefficients, but shares one set of poles. D is real; real poles have
real residues, and complex poles/residues occur in conjugate pairs. The model is
proper (no sE term). It fits complex absolute error, not dB, phase, relative-error
weights, reciprocity, or passivity. There is no delay correction, passivity
repair, automatic order search, multistart, export, or extrapolation option.

Enable `experimental-vector-fitting` explicitly. It adds **no dependencies** and
installs no solver/native backend. The `cli` default feature is independent:

```sh
cargo run --release --features experimental-vector-fitting --example vector_fitting
cargo run --release --no-default-features --features experimental-vector-fitting --example vector_fitting
cargo test --no-default-features --features experimental-vector-fitting --test vector_fitting
```

The [runnable example](../examples/vector_fitting.rs) performs all three
comparisons below. It reads the embedded fixture and writes nothing. Public
analytic workflow contracts are in [tests/vector_fitting.rs](../tests/vector_fitting.rs).

## API and acceptance sequence

1. Parse measured/simulated data with `Network::new`, `from_str`, or `from_bytes`.
   Use `NetworkBuilder` only for synthetic data. Choose training and separately
   withheld samples **before** fitting. Do not interpolate a reference then call
   it an independent measurement.
2. Call `RationalNetworkModel::fit(&training, FitOptions { initial_poles,
   max_iterations })`. Choose `InitialPoles::OneReal`, `TwoReal`, or
   `ConjugatePair { real, imaginary }` explicitly. Seeds use normalized coordinates
   `z = s/(2π f_max) = j f/f_max`, **not Hz or rad/s**.
3. Inspect `model.report().status`. `IterationLimit` returns a diagnostic model,
   **not a successful converged fit**. Rank loss and invalid poles instead return
   structured `FitError`; there is no silent order reduction, fixed-pole fallback,
   regularization, or invented pole identification on a constant through.
4. `model.evaluate(frequency_hz)` only accepts the closed training band. It returns
   a checked `SMatrix` with usual 1-indexed port access. Normalized poles are
   accessible without multiplying by a possibly overflowing `2π f_max`.
5. Call `model.assess(&held_out)`. The separately supplied `Network` must have
   matching rank, S/single-ended semantics and exact reference metadata, finite
   strictly increasing frequencies, and no training-frequency overlap. All points
   must be in-band. Assessment streams through points, reporting aggregate and
   **per-entry** absolute complex RMS, maximum, and relative L2 (absent for a zero
   reference), for both the rational model and the actual training snapshot's
   `Network::sample_at(Linear, Error)`. No holdout enters stopping or relocation.
6. Apply your own RMS **and maximum** error budgets and compare with RI-linear.
   An example-only budget of RMS <=0.01 and max <=0.05 absolute S, plus numerical
   convergence, is printed by the example. It is not a library certificate.

`fixed_pole_baseline` is an explicitly named experimental **non-VF** contrast.
Its status is `FixedPolesNotRelocated`, never `Converged`. It identifies residues
at the seed poles without claiming those poles were learned.

The model clones the validated bounded training network at fitting time, preserving
its actual RI baseline, current reference metadata, and current warnings. Fitting
never mutates source data/metadata. Later edits to the caller's network cannot
change model evaluation, `reference_impedance()`, `source_warnings()`, or its RI
baseline. No extra provenance field is required on `Network` or its constructors
and transformations.

Fit and assessment validate **current** metadata and stored values: S parameter
type, rank 1/2 matching every stored matrix, coherent finite positive references
(including z0), finite complex values, and a finite increasing grid. A current
warning for an ignored `[Mixed-Mode Order]` declaration causes rejection. Other
current warnings are snapshotted, not discarded. Caller-deleted warnings or
consistently retagged public metadata **before** fitting/assessment cannot be
detected. Editing metadata alone is not physical parameter conversion or impedance
renormalization; callers must supply actual single-ended S data with truthful
references. Format/unit presentation strings do not reinterpret the private RI
data or Hz grid.

## Why this is vector fitting

The [orchestrator](../src/vector_fitting.rs) normalizes frequencies by f_max and
all responses by one global maximum absolute component. It does not individually
rescale weak matrix entries. For each channel c, it solves a real stacked complex
least-squares system:

```text
N_c(z) = D_c + sum r_ck phi_k(z)
sigma(z) = 1 + sum a_k phi_k(z)
N_c(z_i) - S_c(z_i) sum a_k phi_k(z_i) ~= S_c(z_i)
```

The numerator coefficients belong to each entry; sigma coefficients are common
to every entry. A real pole uses `phi=1/(z-p)`. For `p=a+jb` and its conjugate,
the two basis functions are `u+v` and `j(u-v)`, where `u=1/(z-p)` and
`v=1/(z-conj(p))`. Real coefficients enforce real realization without fitting
negative-frequency samples or independently approximating phase.

The [private numerical module](../src/vector_fitting/numerics.rs) equilibrates
columns, then uses Householder QR with column pivoting and recomputed trailing
norms. It **does not form normal equations**. Scaled pivots <=1e-11 cause rank
failure. The report's minimum/maximum QR diagonal ratio is only a conditioning
indicator, not a condition-number estimate or an accuracy bound. This small
least-squares module is distinct from the network conversion matrix solver.

After fitting sigma, the algorithm relocates poles to **its zeros**. With at
most two poles these are roots of a real monic quadratic; coefficients are scaled
before the discriminant, and real roots use a cancellation-resistant q/C formula.
This analytic sigma-zero step is not a direct polynomial-denominator fit.
The new topology may switch between two real poles and a conjugate pair.
Right-half-plane roots are reflected and counted. Each step then performs a
**fresh** residue/real-D QR solve at the relocated poles and measures actual
response error, not merely the linearized residual. Fixed-pole least squares
omits the essential sigma solve and relocation; it is not a substitute.

## Explicit operational bounds

| Dimension | Policy |
| --- | --- |
| Matrix | Full known single-ended S, rank 1 or 2; common or per-port finite positive real references |
| Pole order | 1 or 2; one negative real, two distinct negatives, or stable conjugate pair |
| Normalized poles | Real part < -1e-10; magnitude <=1e4; pair separation >=1e-7 × max(1, magnitudes) |
| Training | 8..512 finite nonnegative strictly increasing Hz points; f_max>0 |
| Grid resolution | Normalized total span >=1e-6, every normalized gap >=1e-12 |
| Budget | 1..12 relocation iterations |
| Convergence | Matched unordered normalized motion <=1e-9, max training sigma deviation <=1e-7, **no reflection on final step** |
| Evaluation | Closed training band only; unrepresentable calculations fail explicitly |
| Holdouts | Nonempty finite strictly increasing disjoint in-band grid; streamed errors, no fitting on holdouts |

The largest relocation system has **4096 real rows × 14 unknowns**:
512 samples × 4 entries × 2 components, with 4×3 numerator coefficients and
2 common sigma coefficients. Its flat f64 matrix payload is 448 KiB, plus RHS,
QR workspace, response arrays and the bounded training clone. Residue solves
have at most 12 unknowns. Work is O(rows × columns²) per solve; up to two solves
per iteration. This is a throughput/working-set bound, **not a measured speedup**.
Do not quietly raise the cap to fit delay or a complex measured filter.

## Verified outcomes and limits

The release example, run with `--no-default-features` and the fitting feature,
produced the following results (rounding/last bits can vary by platform). Errors
are absolute complex S over all entries and withheld points.

| Case | Status / iterations | Training RMS / max (VF) | Withheld RMS / max (VF) | Fixed-pole withheld RMS / max | Actual RI-linear withheld RMS / max |
| --- | --- | --- | --- | --- | --- |
| Analytic passive resonator, 41 training / 160 withheld | Converged / 2 | 4.856491e-16 / 1.494683e-15 | 5.824989e-16 / 2.931073e-15 | 1.748292e-1 / 7.503277e-1 | 3.452033e-2 / 2.650614e-1 |
| 12-cycle delay, 81 / 320 | IterationLimit / 12 | 6.264053e-1 / 9.940102e-1 | 6.265572e-1 / 9.941442e-1 | 6.363664e-1 / 9.126405e-1 | 4.583886e-2 / 8.311822e-2 |
| Vendor low-pass fixture, 257 / 256 | IterationLimit / 12 | 5.055543e-1 / 1.137725 | 5.058303e-1 / 1.137659 | 5.798605e-1 / 1.149390 | 7.917015e-3 / 3.134925e-2 |

The synthetic passive resonator is independently evaluated as
`h(z)=0.032z/(z²+0.032z+0.61²)`,
`S=0.8 [[h,1-h],[1-h,h]]`. Its eigenchannels are 0.8 and 0.8(2h-1), with
`|2h-1|=1` on the imaginary axis. The fitted normalized poles are
`-0.016 +/- j0.6097901278308792`, starting from `-0.15 +/- j0.2`.
This analytic passivity argument applies to that reference, not arbitrary fits.

The measured file [LFCN-2352+_Plus25degC.s2p](../files/LFCN-2352+_Plus25degC.s2p)
identifies Mini-Circuits, a LOW PASS FILTER, input power 0 dBm and +25 deg C.
Its header records a pinned scikit-rf source URL and BSD-3-Clause provenance.
The example selects 257 deterministic evenly index-spaced **original untouched
DB/angle rows**, including endpoints, and 256 disjoint original midpoint-index
rows, over 10 MHz..50 GHz. Neither seed nor stopping uses holdouts. This is a
real passive-component measurement comparison, **not independent certification
of passivity or truth at unmeasured frequencies**. No paper data or reference
library is installed or copied. The attempted two-pole model is grossly worse
than interpolation and fails the explicit suitability budget. **There is no
accepted accurate measured fit in this implementation's evidence. Synthetic
accuracy is proven for tested laws; this measured fixture fails suitability.**

Error reports accumulate real and imaginary components using scaled sums of
squares, then normalize by sample/value count only when forming RMS. Relative L2
is computed from the scaled norms **before physical-unit rounding**, not by dividing
rounded RMS values. Thus an absolute RMS may legitimately round to zero while
relative error remains nonzero; `None` means an identically zero reference, not
underflow. Unrepresentable RMS, maximum, or relative error returns `FitError::Numerical`
rather than silently returning infinity. This does not change absolute-error fitting
into relative-error fitting.

The public subnormal regression uses an independent one-real-pole transfer law at
amplitudes 1e-320, 1e-322 and 5e-323. Both training and held-out reported relative
errors agree with two externally normalized sum-of-squares oracles (generating
amplitude and measured component peak), rather than falsely reporting perfect
zero error. Re-running the independent review probe gave these held-out values:

| Generating amplitude | Relative L2 (report and independent amplitude-scaled oracle) | Absolute RMS / maximum |
| --- | --- | --- |
| 1e-320 | 1.5626643249301713e-4 | 0 / minimum subnormal |
| 1e-322 | 1.57739695054446e-2 | 0 / minimum subnormal |
| 5e-323 | 4.501491366063437e-2 | 0 / minimum subnormal |

These quantized observations are not accuracy claims or guarantees of convergence
at all subnormal amplitudes: the probe's amplitude `f64::from_bits(1)` dynamic
signal still fails explicitly with `InvalidPoles`. Its separate 50-case rational
sweep completed with no failures and worst held-out maximum 6.703247e-14.

A fixed-pole minimum-subnormal constant retains a nonzero reference
(`Some(0)` for exact training recovery); against zero holdouts its RMS and maximum
are exactly `f64::from_bits(1)` and its relative error is `None`. Large finite
constant workflows also verify finite RMS without forming an overflowing reference
norm, with explicit errors for unrepresentable ratios or differences.

Public tests additionally recover one-real-pole scalar and two-real-pole full
matrices, different residues/signs/nonreciprocal entries, both topology transitions,
DC, and finite frequency/amplitude rescalings. They require independent exact
held-out transfer-law values. Direct-only, zero and overordered inputs cause
rank errors. Nearly invisible dynamics, repeated/near-coalescent poles, malformed
grids/metadata, invalid budgets and out-of-band assessment are rejected.
An unstable target that stagnates after reflection returns `IterationLimit`:
pole motion alone is not convergence. A one-step exact recovery still returns
`IterationLimit` until sigma and pole-motion confirmation succeed.

## Troubleshooting and scientific boundaries

- **RankDeficient:** try an explicitly chosen smaller order only when physically
  justified. A pure through does not identify dynamic poles. Do not treat a
  pseudoinverse, arbitrary seed poles, or noise-restored rank as identification.
- **InvalidPoles:** repeated/too-close roots and nearly undamped or oversized
  normalized poles are outside this partial-fraction slice; no hidden clamping.
- **IterationLimit:** retain the report as diagnostic evidence; do not ship it
  as a confident fit. More iterations are intentionally not an escape hatch.
- **Converged but poor errors:** the model class may be too small. Convergence
  and stability cannot fix delay, missing modes, noise, or unresolved resonances.
  Use RI-linear interpolation or a separately reviewed modeling tool.
- **Numerical:** finite inputs alone do not guarantee representable arithmetic;
  extreme cancellation/overflow can fail. Scaling tests are evidence for their
  stated cases, not all finite f64 inputs.
- A tiny RMS can hide a narrow missing peak or a weak trace. Inspect maximum and
  per-entry errors, sampling density/noise and domain knowledge. No fit can
  reliably discover an unmeasured resonance from a low residual alone.

The [literature roadmap, section 6](literature-roadmap.md#6-vector-fitting-as-an-explicit-broadband-modeling-subsystem)
remains broader: arbitrary order, delay treatment, backend selection, passivity
assessment/enforcement and simulation exports are **not** implemented here.
The algorithm follows the shared-pole relocation idea described there, with
independently written bounded numerical code rather than copied reference code.
