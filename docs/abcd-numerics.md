# ABCD numerical investigation

This report records the Rust f64 investigation at commit `84f57bc`, before
the shared conversion implementation. Its saved output is historical evidence.
The current example uses `try_cascade`, so invalid inputs now produce errors
instead of the old NaN cascade results. See [the consistency contract](cascade-consistency.md)
for the implemented follow-up. The Bend experiments remain archived outside
this repository; this investigation is self-contained.

## Reproduce

```sh
cargo run --release --example abcd_numerics
cargo test --example abcd_numerics
just fmt-check test doc-check
```

The example compares the existing public conversions, determinant-tracked ABCD,
and direct scattering composition. It reports baseline failures. The example tests assert
candidate accuracy for the defined fixtures. A baseline failure does not make
the experiment fail. This is an accuracy experiment, not a throughput benchmark.

## Why investigate

Both ABCD-to-S implementations reconstruct reverse transmission with
`S12 = 2 * (A*D - B*C) / denominator`. When the two products are large and
nearly equal, subtraction can erase the determinant even with f64 arithmetic.
More exponent range alone does not repair this cancellation.

If the original S parameters are available, store `det = S12/S21`, multiply
determinants during cascade, and recover `S12 = det*S21`. Do not assume a unit
determinant for measured or nonreciprocal networks. An arbitrary ABCD matrix
cannot supply information already lost when its entries were rounded.

For a cascade of two S matrices L and R, the internal connection has denominator
`1 - L22*R11`. Direct scattering composition avoids converting through ABCD
and permits zero forward transmission when the internal connection is solvable.
It can still be ill-conditioned near an internal resonance. A small residual
or agreement between two f64 algorithms is not a universal accuracy guarantee.

The requested -174 to +100 dBm envelope describes absolute power. S parameters
are amplitude ratios. Their attenuation in dB is a different quantity. Boundary
probes here characterize failure modes; they do not impose a new RF requirement.

## Measured results

Recorded on arm64 with rustc 1.98.1, using the release example.
[Full output](abcd-numerics-output.txt) retains every reported case.

| Probe | Expected S12 | Existing Rust result | Candidate result |
| --- | ---: | ---: | ---: |
| Matched 180 dB S-to-ABCD-to-S | 1e-9 | 0 | Tracked: 1e-9 |
| Two matched 90 dB stages, actual `Network::cascade` | 1e-9 | 3.2e-8 | Both: about 1e-9 |
| Two matched 120 dB stages, actual `Network::cascade` | 1e-12 | 0 | Both: 1e-12 |
| Twelve matched 20 dB stages, public conversion composition | 1e-12 | -3.355e-5 | Both: about 1e-12 |
| Zero forward transmission in first stage | Finite scattering result | `Network::cascade` returns NaN | Direct scattering stays finite |

The matched references are analytic products of transmission coefficients.
On the 12-stage chain, maximum relative transmission error was 8.078e-16 for
the tracked candidate and 6.058e-16 for direct scattering. At 120 dB the public
round-trip transmission relative error was already 1.221e-4. The larger
failures therefore follow a measurable loss of accuracy, not just a range cutoff.

Two complex pairs cover nonreciprocity, mismatch, and an internal loop product
of 0.999999. Both candidates agree with an independently arranged, pivoted
internal-wave linear solve in f64. Maximum reflection disagreement in these
two cases was below 1.6e-14. This is algorithm cross-checking, not a high-precision
reference or a proof for all resonant networks.

A 260 dB round-trip boundary probe is rejected because `2*S21 = 2e-13` is
below the public API's absolute tolerance. It is reported separately from the
0..240 dB fixture sweep. Neither range is a promised application operating envelope.

The tracked candidate stores ABCD numerators with a shared denominator and
defers division, in addition to carrying the determinant. It has no exponent
normalization. Its results cannot be attributed solely to determinant tracking,
and it is not ready for arbitrary-range inputs. The direct candidate also has
only experimental validation and exact-zero singularity handling.

Four focused example tests pass. The existing `just test`, `just doc-check`,
and final `just fmt-check` pass. The baseline `just lint` fails on three
pre-existing `useless_borrows_in_formatting` warnings in `src/cli.rs` at lines
302, 309, and 311. No production source was changed to silence them.

## Code review findings at the investigation commit

- `Network::cascade` uses the older `RealImaginaryMatrix` conversion methods.
  Those methods do not have the public matrix API's denominator validation.
- The public conversion methods reject denominator magnitudes at or below
  `1e-12`. For S-to-ABCD this is a cutoff on `2*S21`, not an assessment of
  the requested output accuracy. A nonzero finite ABCD representation can be
  rejected by this policy.
- Both complex division implementations square the denominator components.
  Very large or small finite values can overflow or underflow during division.
  This is a separate range issue, not the determinant cancellation fix.
- Public conversions check input finiteness but do not validate every computed
  output. The legacy cascade also lacks a structured error return.
- Existing conversion tests use absolute component tolerances. Tests for tiny
  nonzero transmission must also bound relative or dB error, or a zero result
  can pass. Reflection near zero needs an absolute tolerance instead.

## Candidate direction

Prefer investigating direct S composition for `Network::cascade`. Retain ABCD
conversion for callers who need circuit transmission matrices. If preserving
ABCD internally is required, use a separate determinant-carrying representation
constructed from S parameters. Do not change the existing public `ABCDMatrix`
fields merely to hide metadata.

Before a production change, define errors for singular connections and nonfinite
outputs. An additive fallible cascade method can preserve the existing method's
return type. Review singularity policy separately from arithmetic. Avoid
clamping invalid outputs or silently forcing reciprocity.

Keep f64. Evaluate dimensionless scaling and robust complex division only with
tests that establish their required operating envelope. Measure performance
separately on representative frequency sweeps and Monte Carlo batches.

## External context

[scikit-rf's connection API](https://scikit-rf.readthedocs.io/en/v1.11.0/api/generated/skrf.network.connect_s.html)
works directly with S matrices. Its
[least-squares connection documentation](https://scikit-rf.readthedocs.io/en/latest/api/generated/skrf.network.innerconnect_s_lstsq.html)
also discusses near-singular internal connections. These are design references,
not the acceptance oracle for this experiment.
