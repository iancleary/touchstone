# Sampled passivity and matrix symmetry

`Network::sampled_quality` is an **opt-in, nonmutating diagnostic**, initially
limited to one- and two-port, single-ended S networks. Parsing active-device data
is still valid. Reporting never clips, repairs, symmetrizes, resamples, or removes
warnings from a network.

## Mental model

For power-normalized waves with positive real reference impedances, a passive
matrix is contractive: `I - SᴴS` is positive semidefinite, equivalently its largest
singular value `sigma_max(S)` is at most one. The corresponding maximum
output/input **power** ratio is `sigma_max²`; the report returns amplitude ratio
rather than squaring it (which could overflow). Common and unequal per-port real
references are supported; no impedance conversion is needed for this norm in the
assumed power-normalized basis. Complex-reference and arbitrary wave-basis
interpretations are not supported.

An elementwise check is insufficient. With every entry equal to `0.6`, equal
coherent port excitation gives `sigma_max = 1.2` and power ratio `1.44` even though
all four individual traces have magnitude below one.

Symmetry is a separate diagnostic: `max |Sij - Sji|`, zero by definition for one
port. In the supported two-port case it is `|S12 - S21|`. This is useful under
conventional reciprocal-network/wave-normalization assumptions, **not a universal
electromagnetic reciprocity test**. A network can be symmetric and active, or
contractive and nonsymmetric. The report therefore selects worst passivity and
worst symmetry locations independently. Exact ties select the earliest point.

A finite grid cannot establish broadband passivity or causality. Exact linear RI
interpolation of contractive endpoint matrices remains contractive by convexity
of the spectral norm, assuming a consistent reference/wave basis. That statement
excludes extrapolation and rounding; the report itself evaluates only stored
samples. It makes no claims about an underlying device between samples.

## Workflow

Use a normal `Network` from a file, string, or byte slice. Only synthetic data
needs `NetworkBuilder`. For example:

```rust
use touchstone::{Network, SampledQualityOptions};

let network = Network::new("files/ntwk1.s2p")?;
for warning in &network.warnings {
    eprintln!("{warning}");
}
let report = network.sampled_quality(SampledQualityOptions { tolerance: 1e-9 })?;
let worst = &report.points[report.worst_passivity.point_index];
println!("sigma={} at {} Hz; status={:?}; raw excess={}",
    worst.sigma_max, worst.location.frequency_hz, worst.passivity,
    worst.excess_above_one);
let asymmetric = &report.points[report.worst_symmetry.point_index];
println!("symmetry residual={} at {} Hz",
    asymmetric.symmetry_residual, asymmetric.location.frequency_hz);
# Ok::<(), touchstone::TouchstoneError>(())
```

Tolerance is mandatory, finite, nonnegative, dimensionless, and absolute. There is
no hidden default and no statistical measurement-uncertainty estimate. Compare
`deviation = sigma_max - 1` with `±tolerance`:

- `BelowBoundary`: deviation is strictly less than `-tolerance`.
- `NearBoundary`: deviation is inside the inclusive band. This is intentionally
  inconclusive, not a silent pass or clipping operation.
- `AboveBoundary`: deviation is strictly greater than `tolerance`.

The implementation compares deviations rather than constructing `1 + tolerance`,
so even an explicitly chosen `f64::MAX` tolerance cannot overflow a threshold.
`excess_above_one = max(sigma_max - 1, 0)` always retains the raw excess; tolerance
is not subtracted. Symmetry residuals are returned directly without a second
classification or an implied universal reciprocity tolerance.

Indexes are zero-based sample indexes; frequencies are in Hz. RF port indexes in
other APIs remain one-based. Results retain source order. Report storage is O(m)
for m samples; each 1/2-port matrix takes constant work with no backend dependency.

## Validation and troubleshooting

The reporting boundary revalidates mutable public metadata instead of trusting
construction alone:

- Parameter must be `S`, rank must be 1 or 2, and every stored matrix must match
  that rank. A mutated rank is not a matrix conversion.
- Grid must be nonempty, finite, nonnegative, and strictly increasing; DC is
  valid. Parsed duplicate, descending, negative, or non-finite grids remain
  inspectable but cannot produce this report.
- All complex RI values must be finite. A valid zero may still have negative
  infinity in the separate dB view; that does not invalidate its complex value.
- Both scalar `z0` and reference metadata must contain positive finite reals.
  Common-reference behavior follows the existing `reference_impedance()` scalar
  compatibility convention: a changed positive `z0` is authoritative. The stored
  common value is also checked for validity, but need not equal `z0`. For per-port
  metadata, length must match rank and `z0` must agree with the first port.
- An unknown-keyword warning for `[Mixed-Mode Order]` is explicitly rejected:
  the parser accepts the declaration but does not preserve its wave semantics.
  Other warnings are retained and remain the caller's responsibility. Do not
  clear warnings or relabel mixed-mode/Y/Z metadata to bypass validation: this
  cannot convert wave bases or parameters.
- If the computed singular value or symmetry residual exceeds finite `f64`
  range, reporting returns `TouchstoneError::SampledQuality` with network name,
  sample index, and frequency rather than returning infinity or NaN.

No general N-port SVD is provided. Use a separately validated numerical backend
for more ports; do not interpret an unconverged power iteration as an upper bound.
No automatic repair is available. An `AboveBoundary` result may reflect an active
device, measurement/calibration problems, source precision, or inappropriate
wave assumptions; investigation belongs to the user.

## Algorithm decision and reproducible evidence

For one port the singular value is `hypot(re, im)`. For two ports, scale every
component by the largest absolute real or imaginary component before forming the
small Hermitian Gram matrix `G = SᴴS`. For its entries `x = G11`, `y = G22`, and
`z = G12`, use

```text
lambda_max = (x + y + hypot(x - y, 2 * |z|)) / 2
sigma_max  = sqrt(lambda_max) * scale
```

Zero matrices take an explicit zero path. Scaling avoids squaring huge or tiny
original values. Only the largest eigenvalue is required, avoiding subtraction
to recover a small singular value. This is an analytic floating-point estimate,
not an interval-arithmetic bound or correctly rounded SVD guarantee. Components
negligible relative to the scale may underflow; reported boundary classifications
must be interpreted with the explicit tolerance. No error bound is claimed.

| Variant | Decision and observed behavior |
| --- | --- |
| Scaled Hermitian formula | Selected: small, dependency-free, uses `hypot` without subtracting nearly equal quantities to obtain the eigenvalue gap. |
| Scaled determinant expression `(sqrt(t+2|det|) + sqrt(max(t-2|det|,0)))/2` | Compared, not selected: cancellation near equal singular values loses their separation. On `diag(1+1e-10,1)`, relative difference from the selected result was `5e-11`. The clamp belongs only to this experiment, not to measured S data. |
| Unscaled Gram matrix | Rejected: squaring `1e300` overflows and squaring `1e-300` underflows. |
| Power iteration | Rejected: an unconverged estimate can miss coherent gain; no certified upper bound. |
| General SVD backend | Deferred: no new dependency or installation; >2 ports explicitly unsupported. |

The maintained [example](../examples/sampled_quality.rs) exercises the public API
and the alternate expression on zero, lossless through, matched attenuator,
coherent gain, complex unitary, rank-one matrices scaled to `1e-300` and `1e300`,
near-degeneracy, and the real `files/ntwk1.s2p` fixture:

```bash
cargo run --offline --example sampled_quality
cargo run --offline --example sampled_quality -- files/ntwk1.s2p
cargo test --offline --test sampled_quality
```

Observed: analytic cases agree with known norms within the example's `2e-15`
relative allowance. Both formulas agreed on these cases except near-degeneracy.
The 91-point fixture's worst norm was `1.0000000007608698` at 9 GHz, consistent
with the chosen `1e-9` tolerance but **not** strictly below one. Its finite source
precision matters. Maximum relative difference between formulas on that fixture
was `1.665e-15`. These bounded observations are not exhaustive numerical proof.
The integration suite additionally exercises subnormals, overflow errors,
independent worst locations, warning preservation, malformed metadata, invalid
grids, unsupported semantics, and tolerance boundaries through stable public APIs.

See [support contract](support.md), [implementation](../src/quality.rs),
[integration checks](../tests/sampled_quality.rs), and the
[literature rationale and sources](literature-roadmap.md#2-sampled-passivity-and-symmetry-diagnostics-not-automatic-repair).
