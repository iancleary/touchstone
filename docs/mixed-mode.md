# Mixed-mode analysis: a basis view, not a new file dialect

A balanced connection is described electrically by single-ended ports, but often
used as differential signals. `MixedModeView` lets you inspect those modes without
changing the source network, discarding warnings, or giving a `Network` ambiguous
port identities. Start with the [README example](../README.md#checked-mixed-mode-views).

## Mental model and source contract

1. Load **known single-ended S data** using `Network::new`, `from_str`, or
   `from_bytes`; use `NetworkBuilder` for synthetic examples only.
2. Define ordered `PortPair { positive, negative }` values using 1-based source
   port numbers. Swapping polarity reverses differential waves, not common waves.
3. Give `MixedModeLayout::new(rank, ports)` the complete row/column order. Each
   pair needs one `ModePort::Differential(pair)` and one `Common(pair)` with the
   **same orientation**. Pairs cannot overlap. Every unpaired source port needs
   one `SingleEnded(port)`. Even an all-single-ended permutation is valid.
4. `MixedModeView::new(&source, layout)` checks source semantics, rank, grid,
   stored RI matrix ranks and finite complex entries, and effective references.
   The immutable borrow prevents changes to source metadata while the view is used.
5. Read `s_matrix_at(index)` or `iter_matrices()`. Frequency-point indexes are
   0-based; returned matrix port indexes are 1-based **output positions**.
   Keep `layout().ports()` and `modal_references()` alongside any owned matrix.

For instance, the five-port output ordering
`[Common(3,4), SingleEnded(5), Differential(1,2), Common(1,2), Differential(3,4)]`
means matrix entry `(5,3)` is differential output on pair (3,4) from differential
input on pair (1,2). Here `(i,j)` is shorthand for `PortPair { positive: i,
negative: j }`, not a Rust tuple accepted by the API.

Construction enforces a nonempty finite, nonnegative, strictly increasing grid
(DC allowed), row/grid agreement, and agreement between public `Network::rank`,
the layout rank, and each stored RI matrix rank. The source matrix storage is
private and square by construction. Public parameter metadata must be `"S"`.
These checks happen even if a network was parsed permissively or public metadata
was edited after parsing. No passivity, reciprocity, or physical-realizability
claim is made.

`Network::reference_impedance()` is authoritative: common references reflect the
current public `z0`, while per-port references use their complete vector. Every
source reference must be positive finite real and there must be exactly one per
source port. Within each pair, values must be **exactly equal**, not merely close.
Different pairs and unpaired ports may have different references. `2R` and `R/2`
must also be representable as positive finite `f64` values.

## Derivation and polarity

The implemented equations come directly from the
[local Touchstone 2.1 specification](touchstone_ver2_1.pdf), physical/printed pages
20–23 ([published source](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)).
Page 20 defines:

```text
Vd = Vp - Vn          Id = (Ip - In)/2
Vc = (Vp + Vn)/2      Ic = Ip + In
```

With equal single-ended references `Rp = Rn = R`, choose physical modal references
`Rd = 2R` and `Rc = R/2`. Substituting into `a = (V + RI)/(2 sqrt(R))` and
`b = (V - RI)/(2 sqrt(R))` gives the page 21 relations:

```text
ad = (ap - an)/sqrt(2)     bd = (bp - bn)/sqrt(2)
ac = (ap + an)/sqrt(2)     bc = (bp + bn)/sqrt(2)
```

Place these sparse rows into `T` in the caller's explicit output order; an
unpaired port has an identity row. Completeness and disjointness make `T`
orthogonal. Thus `bm = T b = T S a = T S T^T am`, so:

```text
Smodal = T Ssingle T^T
```

This transforms complex waves, **not dB trace differences**. No unequal-reference
(nonunitary) transformation, complex reference, frequency-dependent reference,
renormalization, or modal Y/Z conversion is provided. Applying existing scalar
S/Y/Z conversions to a returned mixed-mode matrix is generally inappropriate:
its physical references differ by mode.

[Bockelman and Eisenstadt (1995)](https://doi.org/10.1109/22.392911) is the
foundational literature reference; this implementation relies on the inspected
IBIS definitions, not a claim that the paper's full text was reviewed. See
[literature roadmap §4](literature-roadmap.md#4-mixed-mode-analysis-with-explicit-port-pairing-and-polarity).

## The file boundary is intentional

Touchstone pages 22–23 define `[Mixed-Mode Order]` as the matrix's row/column
identity vector. The parser still does **not** implement this keyword; it records
`UnknownKeywordIgnored`. A view rejects sources with that warning, including
normalized case/hyphen/whitespace variations, to avoid transforming mixed-mode
values twice. Other warnings remain accessible unchanged through `view.source()`.
Callers must still establish that the input is single-ended: missing declarations,
or a caller deleting warnings/changing parameter metadata, cannot be detected.

A view has no `save`, `write_touchstone`, `to_touchstone_string`, or conversion to
`Network`. The source remains available and unchanged, but writing the source
writes its original **single-ended** matrix, not the view. A returned `SMatrix`
is an owned numeric snapshot, not a serializable modal network. Keep its layout
and physical references with it; feeding it to a builder would lose that meaning.

In particular, `[Reference]` in a mixed-mode file still describes **single-ended**
references. Writing modal 100/25-ohm values as though they were the original
50/50-ohm pair's `[Reference]` would be incorrect. Parser/writer mixed-mode
support is deferred until descriptor and reference metadata can be preserved
coherently. The [support table](support.md) retains that limitation.

## Numerical behavior and physical evidence

The implementation in [`src/mixed_mode.rs`](../src/mixed_mode.rs) evaluates each
entry from at most four sparse terms, rather than dense matrix multiplication.
Each real/imaginary component is normalized by the largest contributing absolute
component before summing, then accumulated with Neumaier compensation. Scaling
avoids overflow in cancellable intermediate sums; compensation retains small
normalized terms that ordinary summation can lose when larger terms cancel.
Two modal coefficients multiply to exactly signed `1/2`, avoiding upward rounding
of `sqrt(1/2)^2` at the overflow boundary. The final rescaling and every output
complex entry are checked: an unrepresentable result returns `MixedModeError`.
Construction does not compute output matrices, so such an error can occur during
iteration even after successful view creation.

For example, real `S = [[1e300, 1], [-1e300, 1]]` with ordering
`[Differential(1,2), Common(1,2)]` has analytic `Scc = 1`. Ordinary normalized
summation returned `0.5`; compensated normalized summation returns `1` in the
public API regression. This fixes avoidable accumulation loss, not all loss of
small contributions.

This is bounded `f64` arithmetic, not arbitrary precision: severe cancellation
can lose relative accuracy; normalization can underflow contributions separated
by extreme dynamic range; rounding near the representable boundary can affect
acceptance; tiny outputs can underflow to zero. No uniform error bound or
passivity enforcement is promised. The output finite guard is not an accuracy
certificate. Reference underflow to zero and overflow to infinity are rejected.

[`tests/mixed_mode.rs`](../tests/mixed_mode.rs) checks public outcomes:

- Balanced reciprocal paths 1↔3 and 2↔4, both unity, produce unity differential
  and common transmission and zero cross-mode conversion for pairs (1,2)/(3,4).
- For path gains `a = .8 + .2j` and `b = .4 - .6j`, the same-mode transmission
  is `(a+b)/2 = .6 - .2j` and cross-mode transmission is
  `(a-b)/2 = .2 + .4j`. Reversing only the second pair flips its differential
  response and stimulus signs while leaving its common response unchanged.
  These are analytic complex expectations, not a round-trip-only check.
- A generalized three-port ordering retains a single-ended port, complex
  coupling, and separate references; different pairs may use 50 and 75 ohms.
- Large cancelling real entries preserve the literal analytic `Scc = 1` above;
  existing balanced, asymmetric complex, and unpaired cases protect ordinary use.
- Invalid coverage/polarity/bounds/overlap, mutable public metadata, unequal or
  unrepresentable references, parser warnings, invalid grids, and extreme
  components exercise rejection and finite-result behavior.

## Lazy view versus explicit owned cache

| Choice | Retained transformed data | Repeated access | Decision |
| --- | --- | --- | --- |
| Borrowed lazy view | Layout/references O(N); no transformed matrices | Recompute each O(N²) matrix | Production API; useful for selective reads/streaming |
| Caller-owned `Vec<SMatrix>` collected from the view | O(F N²) complex payload plus allocation overhead | Read already computed matrices | Ordinary caller code; no competing production type |

Both choices scan the source once when creating the view (O(F N²) validation).
Each lazy matrix read has O(N²) output storage, bounded stack scratch per entry,
and a `Vec` allocation per output row plus its outer vector. A view is not an
allocation-free matrix reference. The original `Network` already owns all source
data; the lazy choice avoids **additional retained transformed** matrices only.
Layout validation itself is O(N²); this slice targets existing measured N-port
workflows, not unbounded huge ranks.

Run the real comparison:

```sh
cargo run --offline --release --example mixed_mode
```

[`examples/mixed_mode.rs`](../examples/mixed_mode.rs) first compares ordinary and
Neumaier normalized summation on selected 1–4-term inputs. It asserts analytic
expectations and checks the cancellation result through the real public view.
The maintained experiment produced:

| Input (weight 1/2 unless noted) | Ordinary | Neumaier | Analytic |
| --- | --- | --- | --- |
| `[.75]`, weight 1 | .75 | .75 | .75 |
| `[.8, .4]` | .6000000000000001 | .6000000000000001 | .6 |
| `[.5, -.25, .75]` | .5 | .5 | .5 |
| `[.5, -.25, .75, 1]` | 1 | 1 | 1 |
| `[1e300, 1, -1e300, 1]` | .5 | 1 | 1 |
| `[MAX, MAX, MAX, -MAX]` | MAX | MAX | MAX |
| `[MAX, MAX, MAX, MAX]` | infinity | infinity | outside finite range |
| `[1e300, 1e-100, -1e300, 1e-100]` | 0 | 0 | 1e-100 |

`MAX` is `f64::MAX`. The overflow experiment is a scalar comparison; existing
public integration coverage verifies that the view rejects non-finite outputs.
The last row deliberately records scaling underflow, which compensation cannot
repair. These selected cases justify choosing compensation without claiming a
universal accuracy improvement or an exhaustive numerical study. No alternative
production type or dependency is introduced.

The same example generates 2,000 asymmetric
four-port points, measures 20 complete traversals, and instruments transform
counts at the public read boundary **only in the example**. It compares full
public matrices against the cache on three additional traversals, outside the
reported timings/counts. A local optimized run produced:

```text
source validation: 43.333 µs
lazy: 10.943583 ms, 40000 transforms
cache build: 532.542 µs, 2000 transforms; cached reads: 255.584 µs
retained transformed complex payload: lazy 0 bytes; cache 512000 bytes
full matrix equality: 3 additional traversals passed
```

These timings are one workload on one machine, not a universal performance
claim or a benchmark gate. Cache traversal reads existing matrices; lazy traversal
includes transformation/allocation/drop, deliberately reflecting those public
workflows. Payload counts exclude vector/allocator overhead and temporary output
matrices. Use a caller cache when repeated full access justifies retained memory;
selective access or a single pass does not establish a need for an owned modal
network type. The example requires no new dependency or external service.

## Troubleshooting and verification

- `InvalidLayout`: list exactly rank output identities and include both modes for
  each identical ordered pair; remove overlaps rather than silently dropping ports.
- `UnsupportedSource`: confirm the original data is single-ended S. Do not clear
  an ignored mixed-mode warning just to bypass the guard.
- `RankMismatch` or numerical matrix-rank failure: public metadata edits cannot
  change stored matrices; reconstruct from correctly declared data.
- `InvalidReference`: use effective references, check each vector length/pair,
  and check physical modal representability. Unequal pairs need a different,
  currently unsupported transform, not a relaxed tolerance.
- `Numerical`: inspect its source `TouchstoneError`; invalid grids fail at view
  creation, while output overflow can fail during lazy reads.

Focused checks: `cargo test --offline --test mixed_mode --test readme_examples`.
Repository gates: `cargo fmt -- --check`,
`cargo clippy --offline --all-targets --all-features -- -D warnings`,
`cargo test --offline --all-features -- --test-threads=1`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --offline --all-features --no-deps`.

Implementation throughput checkpoint:
- **Blocking first steps:** spec equations and source invariants reviewed before edits.
- **Independent workstreams:** n/a; API, physical evidence, and docs are one coupled slice.
- **Shared mutable state:** immutable source borrow; all worktree/Git writes serialized
  (no Git writes performed by the implementation worker).
- **Smallest safe decomposition:** one leaf owner, no nested delegation; self-review
  here, independent verification/publication reserved for the parent.
