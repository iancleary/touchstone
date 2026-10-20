# Changing a common real reference impedance

## Mental model and API

Reference impedance is part of the wave definition, not a load added to a
circuit. Editing `Network::z0` changes metadata; it does **not** transform the
S parameters. Use the explicit [`Network::renormalize`](../src/renormalization.rs)
operation to represent the same circuit in a new reference environment:

```rust
use touchstone::Network;
let measured = Network::from_str("load.s1p", "# Hz S RI R 50\n1 0.2 0\n")?;
let at_75 = measured.renormalize(75.0)?;
assert!(at_75.try_s_ri_at(0, 1, 1)?.re.abs() < 1e-14);
assert_eq!(at_75.z0, 75.0);
# Ok::<(), touchstone::TouchstoneError>(())
```

This example is executable in the method's rustdoc. A 75-ohm load has reflection
`0.2` at a 50-ohm reference and zero at 75 ohms. Conversely, a matched 50-ohm load
has reflection `-0.2` at 75 ohms. Ideal opens/shorts stay `+1`/`-1` at ordinary
reference ratios.

The scope is deliberately **one common positive finite real reference**, at all
ports and frequencies, to another. Per-port references are rejected even if
all their entries are equal. Complex/frequency-dependent references, mixed-mode
wave conventions, implicit resampling, and automatic reference changes during
cascade are not supported. An ignored `[Mixed-Mode Order]` warning blocks the
operation: a parser warning is not evidence that the stored waves are
single-ended. The guard checks current public warnings, normalizing ASCII case,
hyphens, and whitespace just as the parser does; it preserves the original keyword
in the error. It does not recover provenance if a caller removes or replaces the
warning. Other parser warnings are retained, not silently removed.

The effective common source reference follows `network.reference_impedance()`
(which uses public `z0` for `Common`, consistent with existing conversions).
Both stored reference fields are updated on a successful reference change.
Same-reference calls validate first and return an unchanged clone, preserving
all original fields/caches rather than introducing solve rounding.

## Direct solve, not an impedance detour

For common real references, set

```text
g = (new - old)/(new + old)
N = S - g I
D = I - g S
S_new D = N
```

The implementation uses the existing [`numerics::solve_right`](../src/numerics.rs)
with `operation`, absolute tolerance `1e-12`, and diagnostics disabled. There is
no second solver, explicit inverse, Z intermediate, new numerical dependency,
or coupling to quality/mixed-mode/connection features. See the shared
[numerical solve policy](numerical-solves.md) for transpose order and LU details.

S→Z requires inverting `I-S`; it fails for an ideal open. Direct renormalization
avoids that artificial singularity at modest ratios. It still rejects a singular
`I-gS`, for example `S=2`, old=50, new=150 (`g=0.5`). Error operation text includes
source name, point index, frequency, and both references. All denominator LU
pivots must have finite magnitude **strictly greater than `1e-12`**. This inherited
absolute policy is scale dependent, not a condition estimate or accuracy budget.
For an open moved from 1 to `1e13` ohms, representable `g` still produces a
roughly `2e-13` denominator and is rejected. No pseudoinverse or regularization
hides the failure.

### Range limits and alternative investigated

Evaluate `g` without forming `new+old`: let `scale=max(old,new)`,
`ratio=min(old,new)/scale`, then use
`g=((new-old)/scale)/(1+ratio)`. Positive finite references have a finite
difference; this ordering also avoids subtracting separately rounded normalized
references when they are close. Thus old=`1e308`, new=`1.5e308` works even though
the original sum would overflow. Subnormal reference examples are also exercised.

This is **not complete f64 reference-range support**. For sufficiently extreme
ratios, `g` rounds to `+1` or `-1` (including ratios of `1e17` and `1e600`).
The public operation returns `UnrepresentableReferenceRatio { old_z0, new_z0 }`
for the entire network rather than silently treating a finite reference as zero
or infinity. Even a matched source could otherwise appear to succeed with a
rounded endpoint while an ideal open/short becomes falsely singular.
Representable `g` also loses accuracy near these endpoints; finite output is not
a forward-error guarantee, and this API does not promise small relative error
for near-zero result components.

The runnable example investigates scalar direct wave scaling:

```text
a = (old/scale) (1+S)
b = (new/scale) (1-S)
S_new = (a-b)/(a+b)
```

At old=1, new=`1e17`, this can preserve ideal open/short results despite rounded
`g`. But old=`1e-300`, new=`1e300` underflows `old/scale` to zero and gives `0/0`
for an open; the reciprocal case fails for a short. General multiport scaling
also needs careful cancellation/scale and pivot-policy analysis. This probe is
**not** a second production implementation or proof of universal failure of all
wave-based algorithms. We chose the bounded direct-g contract with explicit
rejection over an unverified larger-range claim. Wider dynamic range needs a
separate, independently validated algorithm and numerical policy.

## Metadata and workflow

The method validates source S semantics, finite positive references, exact
supported source metadata (`RI`/`MA`/`DB` format and `Hz`/`kHz`/`MHz`/`GHz` unit),
nonempty strictly increasing finite nonnegative frequency grid, grid/data lengths,
row frequencies, stored versus public rank, and finite square source matrices.
These checks run before the same-reference shortcut. Public metadata edits such
as `format = "bogus"` or `frequency_unit = "bogus"` return `UnsupportedFormat`
or `UnsupportedFrequencyUnit` rather than producing an unsafe-to-serialize result.
Case/whitespace variants are rejected, not silently canonicalized, because the
unchanged writer expects canonical tokens. Although the builder/writer can emit
`THz`, renormalization rejects it: the option-line parser does not recognize it
and reparsing would change the frequency scale. This deliberately narrower
contract does not change the parser or writer.
It creates new RI matrices and rebuilds MA/DB caches using the existing
`parsed_data_line_from_ri_matrix`, then clones metadata and replaces only the
samples and both reference fields. Zero magnitudes naturally have `-inf` dB;
finite-result checks apply to RI values, not a requirement of finite log zero.
The source is never mutated, including when a later sample fails.

Name, comments before/after the option line, warnings, data format, parameter,
resistance token, frequency unit, and Hz grid remain unchanged. No builder or
serialization roundtrip is used in the transformation itself. Saving and parsing
the transformed network preserves the **supported** Touchstone metadata and
values, not the original document's exact spelling/layout. The writer does not
serialize parser warnings or ignored unknown keywords: warnings remain observable
on the in-memory result but a newly parsed saved document need not repeat them.
Do not use saving as a warning archival mechanism.

## Evidence and reproducible verification

```sh
cargo test --offline --test renormalization
cargo run --offline --release --example renormalization
cargo fmt --all -- --check
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --all-features -- --test-threads=1
RUSTDOCFLAGS='-D warnings' cargo doc --offline --all-features --no-deps
git diff --check
```

[`tests/renormalization.rs`](../tests/renormalization.rs) exercises public APIs:
analytic loads, ideal boundaries, same-reference identity, a `75+j25` ohm load,
and an independent two-port T-resistor circuit. The circuit has series arms
75/25 ohms and common shunt arm 25 ohms: Kirchhoff's laws give
`Z=[[100,25],[25,50]]`. At 50 ohms the expected S matrix is
`[[7,4],[4,-1]]/23`; at 75 ohms it is `[[2,3],[3,-4]]/17`.
The target reference is computed from the circuit equations, not merely a
roundtrip. Tests also cover metadata serialization, cache views, retained warnings,
invalid inputs, contextual singular/cutoff failures, mixed-mode/per-port rejection,
extreme sums/ratios, and unchanged sources.

The [release example](../examples/renormalization.rs) compares direct S with
actual S→Z→S APIs for the loads/circuit and dense complex synthetic networks at
ranks 2, 8, and 32. It also runs the scalar wave-scaling probe and a compact
throughput checkpoint (32 points, 10 repetitions). Timing includes direct
metadata/cache reconstruction versus detour matrix results only; these are
single elapsed totals, not equal-work benchmarks, statistical speedup claims,
accuracy guarantees, or measurements of real instrument data. Dense comparisons
are cross-checks, not independent circuit references. There is no external
arbitrary-precision oracle or installed backend.

Verification passed formatting, all-target/all-feature Clippy, focused
renormalization/README tests (10 and 17), the full serial all-feature suite
(318 tests including 29 doctests), warnings-as-errors API docs, and
`git diff --check`. No existing numerical threshold or test tolerance was weakened.

Separate review confirmed physical complex series-impedance and nonreciprocal
nilpotent-matrix cases. It also found that invalid public format/unit metadata
could pass transformation and then break serialization. The added regressions
failed before the correction and now pass for both identity and changed-reference
calls. Current coverage also checks case/hyphen/whitespace warning edits and
supported format/unit serialization roundtrips.

Run full suites in different worktrees sequentially: some existing save/load
tests share fixed temporary filenames. Concurrent runs can collide even when
source and Cargo target directories are separate. Run `just check` on a clean
checkout for the package gate as well as the checks above.

One release run on arm64, rustc `1.99.0 (b940084d7 2026-09-28)`:

| Rank / 320 points | Direct, including caches | S→Z→S matrices only |
| --- | ---: | ---: |
| 2 | 434 µs | 464 µs |
| 8 | 2.00 ms | 2.19 ms |
| 32 | 23.8 ms | 30.7 ms |

These are the unequal-work, single-run observations described above, not a stable
performance claim. The example prints fresh totals each time it is run.

Sources: [literature roadmap §3 and bibliography](literature-roadmap.md#3-renormalize-real-reference-impedances-without-an-unnecessary-z-detour),
Marks and Williams (1992) §§3.7–3.8 (wave/reference transformations), Kurokawa
(1965) (power-wave convention), and [Touchstone 2.1](touchstone_ver2_1.pdf), p. 11
(real positive reference values; physical p. 11 extracted locally with system
PDFKit). That page explicitly says real-valued resistances, single-ended ports,
no complex/imaginary impedances, and option-line fallback when `[Reference]` is
absent. The parser's existing common-reference convenience is not a new claim
of full standard conformance. The roadmap records literature inspection limits;
this slice uses its documented scalar-real contract, not a new general complex
wave convention.

### Throughput checkpoint / ownership

- [x] **Blocking first steps:** clean status checked on supplied numerical-solves
  base; numerical contract, implementation, roadmap §3, and Rust guidance read
  before implementation. Analytic/range probes run before finalizing the policy.
- [x] **Independent workstreams:** n/a — one numerical feature, code-coupled
  implementation/example/tests/docs; depends solely on the existing numerics base.
- [x] **Shared mutable state:** isolated worktree, single owner; no Git writes,
  installations, publication, background jobs, or outside-worktree edits.
- [x] **Smallest safe decomposition:** one narrow module and two contextual error
  variants; no broad metadata refactor or changes to other feature slices. Leaf
  implements directly; parent owns independent review and packaging.
