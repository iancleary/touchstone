# Single-pair N-port connections and reflection terminations

## Mental model: eliminate waves, do not slice ports

[`Network::connect`](../src/connection.rs) joins one explicitly selected port of
one sampled single-ended S network to one port of another. The two joined ports
become internal wave variables; all other ports remain externally accessible.
The result is a `ConnectedNetwork { network, surviving_ports }`, not just a
matrix whose port identities must be guessed.

Port arguments are **1-indexed**. Output ports are left ports ascending excluding
the joined port, followed by right ports ascending excluding the joined port.
Each `SourcePort { side: NetworkSide, port }` identifies the original input port.
For example, joining left port 2 of a three-port to right port 1 of a two-port
leaves `[Left:1, Left:3, Right:2]`. Identity is relative to this operation's two
inputs, not a persistent global topology graph across later connections.

A one-port network supplies its sampled reflection coefficient as an ordinary
S11 and participates in the **same operation**. A matched load has Gamma=0, an
open Gamma=+1, a short Gamma=-1. Removing a row and column without wave elimination
assumes a matched load; it does not model an open or an arbitrary termination.
Repeated `connect` calls compose separate networks; this API does not implement
self-connections, simultaneous multiple joins, or a sparse circuit simulator.

## Runnable usage

This minimal through/load example is also executed in the method's rustdoc:

```rust
use touchstone::{Network, NetworkSide, SourcePort};
let through = Network::from_str("through.s2p", "# Hz S RI R 50\n1 0 0 1 0 1 0 0 0\n")?;
let load = Network::from_str("load.s1p", "# Hz S RI R 50\n1 0.2 0\n")?;
let connected = through.connect(&load, 2, 1)?;
assert_eq!(connected.surviving_ports, vec![SourcePort { side: NetworkSide::Left, port: 1 }]);
assert_eq!(connected.network.s_matrix_at(0)?.get(1, 1)?.re, 0.2);
# Ok::<(), touchstone::TouchstoneError>(())
```

For measured inputs, use `Network::new`, `from_str`, or `from_bytes` and retain
warnings. Use `NetworkBuilder` only for generated data. Serialize the result with
`connected.network.to_touchstone_string()` or `save`; the result has ordinary
single-ended S data, not an unmodeled modal basis.

## Boundary contract and metadata

- Source ranks and result rank are explicitly limited to **1–127**. The result
  rank is `left.rank + right.rank - 2`; joining two one-ports is rejected because
  no external port remains. Arithmetic uses checked ranges before conversion;
  large public ranks cannot wrap into a smaller matrix rank.
- Both frequency/data vectors must be nonempty, aligned, and finite, with a
  strictly increasing nonnegative Hz grid. Stored row ranks and frequencies must
  agree with public metadata. All source RI entries must be finite. Matrices and
  frequency vectors are private, but public rank/metadata can be changed, so
  `connect` revalidates at the boundary.
- Frequency values must match **exactly in Hz**, including point counts. GHz vs
  MHz display units are accepted when their stored Hz values agree. There is no
  implicit interpolation; call `resample` explicitly for a different grid.
- Common and per-port **positive finite real** references are accepted. Per-port
  vectors must have exactly rank entries. Scalar `z0` must agree with the first
  reference. Only the joined references must equal; the surviving references
  need not equal one another. This is not automatic renormalization.
- Non-S inputs and sources carrying an ignored `[Mixed-Mode Order]` warning are
  rejected. Such a warning means single-ended port semantics were not established.
  Other parser warnings are retained, not interpreted as physical validation.
- The output name is `Connected(left-name:left-port,right-name:right-port)`.
  It uses the left canonical display unit and format (`RI`, `MA`, or `DB`), S
  parameters, and `R`. Comments before/after the option line and warnings are
  concatenated left first, preserving their original strings/source locations.
  Generated `THz` display metadata is accepted and preserved, but the existing
  option-line parser does not correctly recognize THz input; it remains an
  unsupported serialization round trip (see [support](support.md)).
  References follow survivor order: collapse to `Common` if all equal, otherwise
  use `PerPort`; `z0` is the first surviving reference. Parser warnings remain
  in-memory diagnostics, not serialized keyword semantics.

`try_cascade`, `cascade`, multiplication, and `cascade_ports` keep their existing
contracts, including common-reference and equal-display-unit requirements. This
additive API does not reroute them through LU or change their failure policy.

## Internal-wave solve and numerical limits

Partition the conceptual block-diagonal source S matrix into external (`e`) and
internal (`i`) ports. The ideal connection swaps the two internal waves with P:

```text
a_i = P b_i
(I - P S_ii) X = P S_ie
S_effective = S_ee + S_ei X
```

With internal order `[left joined, right joined]`, the denominator is
`[[1, -right.Srr], [-left.Sll, 1]]`. The RHS has two rows and one column per
surviving port. Production factors it with the existing
[`ComplexLu::factor` / `solve`](../src/numerics.rs), solving all RHS columns at
once. There is **no second solver, explicit inverse, or giant block-diagonal
allocation**. Each external row couples to only one internal member, so the
Schur update needs one complex product per entry, accumulated with f64 fused
multiply-add. Work is O((n+m)²), output/cache storage O((n+m)²), and the internal
solve has just two unknowns regardless of external rank. Source validation and
rebuilding the RI/MA/DB views are included in the public operation.

Every LU pivot magnitude must be finite and **strictly greater than `1e-12`**.
This fixed absolute policy is scale dependent, not an accuracy bound or a
condition estimate. There is no pseudoinverse, regularization, or silent repair.
An ideal open on an internally unit-reflecting port is singular even when an
external path is decoupled: no unique internal-wave solution is selected. A
near-resonant connection below the cutoff also errors. For extreme active
reflections, pivot ordering can make acceptance differ from the scalar cascade
denominator test. The legacy cascade test remains unchanged.

Numerical failures return `TouchstoneError::Connection { operation, point_index,
frequency_hz, source }`; the nested cause distinguishes singular pivots and
nonfinite arithmetic. `root_cause()` traverses this wrapper, and the standard
error source chain retains it. Boundary failures use existing shape/index errors
or `InvalidConnection { reason }` for connection-specific semantic mismatches.
Source finite-value failures also carry sampled frequency context.

All output RI entries must be finite. The shared LU's intermediate calculations
must fit f64; FMA reduces some update rounding but is not extended precision.
Finite inputs whose intermediate/final results overflow can fail even if a
rearranged exact expression would be representable. Underflow can erase tiny
transmission, and near-cancellation can lose relative accuracy. Existing MA/DB
cache conversion range limits still apply; zero magnitude has -infinite dB by
convention. No forward-error certificate, passivity test, broadband physical
validation, or guarantee for arbitrary near-resonant active networks is claimed.

## Independent references and measured experiment

The derivation follows [roadmap section 5](literature-roadmap.md#5-general-n-port-connections-via-internal-wave-elimination),
not code adapted from scikit-rf or an unread paper. See
[numerical solves](numerical-solves.md) for the shared primitive and its limits.

[`tests/nport_connection.rs`](../tests/nport_connection.rs) exercises parsed and
generated inputs, multi-point RI/MA/DB serialization, distinct display units, per-port references,
comments/warnings, external identity, all twelve port pairs between dense 3/4-port
networks, invalid public metadata, malformed grids, rank boundaries, finite
extremes, and exact/near singularities. The ideal equal three-port splitter has
`Sjj=-1/3`, `Sjk=2/3`. Loading port k gives the independently evaluated response

```text
S'ij = Sij + Sik Gamma Skj / (1 - Skk Gamma)
     = Sij + (4/9) Gamma / (1 + Gamma/3)
```

Matched/open/short/complex loads are checked with absolute error near reflection
nulls. Nonreciprocal two-port composition is compared against the existing
scalar cascade for zero transmission and down to 1e-150 transmission, using
relative error for tiny responses. These are analytic/independent-expression
references, not arbitrary-precision external solver evidence.

Run the maintained experiment (no new dependencies):

```sh
cargo run --offline --release --example nport_connection
cargo test --offline --test nport_connection
```

The [example](../examples/nport_connection.rs) compares production block solves
to existing scalar cascade, checks the analytic splitter, and retains a scalar
single-denominator N-port composition **only as an experimental baseline**. On
arm64, rustc `1.99.0 (b940084d7 2026-09-28)`, release profile:

| Source rank each | Public block connection, mean | Matrix-only scalar, mean | Maximum absolute entry difference |
| --- | ---: | ---: | ---: |
| 2 | 1.215 µs | 99 ns | 4.337e-19 |
| 8 | 6.705 µs | 1.651 µs | 1.399e-17 |
| 32 | 89.889 µs | 25.250 µs | 1.388e-17 |

Each mean is 100 repeated single-frequency calls with allocation and no
statistical controls. Public timing includes validation, metadata, and cache
construction; scalar timing uses already extracted matrices and omits those
costs. This is a throughput checkpoint, **not a fair solver speed ratio**, a
universal accuracy envelope, or a claim of speedup. Two-port maximum absolute
differences were 0, 5.594e-17, 2.379e-117, and 3.792e-167 for transmission
0, 0.8, 1e-100, and 1e-150. Analytic splitter probes passed. The singular open
termination printed the operation, point 0, 1e9 Hz, and nested pivot failure.

| Choice | Decision |
| --- | --- |
| Shared two-internal-wave LU | Selected: reusable primitive, explicit resonance policy, multiple RHS, no rank-sized dense factorization. |
| Scalar denominator composition | Retained only in the experiment: simpler/faster arithmetic is possible, but a second production primitive would duplicate numerical policy. |
| ABCD conversion | Rejected: does not generalize this contract and introduces deep-loss failure modes. |
| Silent slicing / pseudoinverse | Rejected: changes load physics / selects a different internal-solution policy. |
| Implicit interpolation or renormalization | Rejected: changes source samples or wave meaning without an explicit user decision. |
| Quality, renormalization, mixed-mode features | Sibling work, not prerequisites or bundled behavior. |

## Verification and throughput checkpoint

Required local checks are listed here; package/release validation remains owned
by the parent after review and commit because the packaging gate needs a clean
checkout:

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --offline --all-features --no-deps
cargo run --offline --release --example nport_connection
git diff --check
```

- [x] **Blocking first steps:** clean status and numerical-solves base inspected;
  supplied baseline `7277e17` had `just check` green; contracts and guidance read.
- [x] **Independent workstreams:** n/a — connection algorithm, public contract,
  experiment, tests, and documentation are one code-coupled slice.
- [x] **Shared mutable state:** one owner in this worktree; no Git state writes,
  installations, background jobs, or outside-worktree modifications.
- [x] **Smallest safe decomposition:** one connection module using the existing
  solver; no recursive delegation or second production numerical primitive.

All six commands passed on the final implementation. The all-features test run
passed **315 tests** (148 library, 138 integration, 29 doctests), including eight
connection integration tests, with no failures. No dependencies, existing test
expectations, or solver/cascade tolerances were changed. The final release
experiment passed every accuracy assertion. No package/publication step ran.

A multi-point THz-input probe also confirmed the already documented option-line
parser limitation; final checks use supported parsed units and independently
verify generated THz display preservation rather than claiming a THz round trip.

The first focused test run caught warning-keyword normalization (parser converts
hyphens to spaces), now corrected. An inappropriate relative assertion at a
reflection null was changed to an absolute-error assertion; tiny-transmission
cascade comparisons retain relative checks. Self-review is not independent
review: parent review remains required. Reuse the existing RF literature review
skill for further research; no duplicate skill or deployment is introduced.
