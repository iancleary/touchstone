# Cascade consistency

Network cascades and public matrix conversions use the same checked f64 ABCD
implementation. The private legacy conversion methods delegate to it too.
This change does not introduce determinant tracking, direct scattering, or f32.

## Error contract

Use `Network::try_cascade` to receive `Result<Network, TouchstoneError>`.
The existing `cascade` method delegates to it and panics on error. Multiplication
and `cascade_ports` retain their signatures and use that same path. Existing
valid, aligned two-port inputs keep the same connection order: left port 2
connects to right port 1.

Inputs must be finite two-port S-parameter networks with the same positive
common real reference impedance, frequency unit, and frequency grid. Empty,
misaligned, or malformed data is rejected. The old implementation could
truncate unequal grids and retain an inconsistent output frequency vector.
Resample explicitly before cascading different grids.

S-to-ABCD and ABCD-to-S validate computed matrix entries, in addition to inputs.
A successful conversion does not contain NaN or infinity in its complex entries.
Zero magnitude can legitimately produce negative infinity in a derived dB view;
that is distinct from a nonfinite complex S parameter.

The existing absolute denominator cutoff of `1e-12` remains. It applies to
`2*S21` on conversion to ABCD and to `A+B/z0+C*z0+D` on conversion to S.
This is a singularity policy, not an error bound. No automatic clamp, forced
reciprocity, or new promise of accuracy near an ill-conditioned conversion is added.

## What the consistency tests demonstrate

| Property | Concrete example | Expected result |
| --- | --- | --- |
| Through identity | Add an ideal through before or after a complex network | All four S entries remain unchanged within tolerance |
| Matched loss | Cascade matched 3 dB and 6 dB pads | 9 dB loss in both directions |
| Matched gain | A 20 dB amplifier followed by a 6 dB pad | 14 dB forward gain; reverse transmission follows its own specified value |
| Reciprocity | Cascade complex reciprocal passive two-ports | S12 and S21 agree within tolerance |
| Nonreciprocity | Give an amplifier different forward and reverse transmission | The two values stay distinct; no reciprocity assumption |
| API agreement | Convert two matrices, multiply ABCD, and convert back | Same result as the network cascade |
| Grouping | Compare `(A cascade B) cascade C` with `A cascade (B cascade C)` | Same physical stage order, agreement within tolerance |
| Round trip | Convert an ordinary complex S matrix to ABCD and back | Recover all four S entries within tolerance |

These are ordinary gain, attenuation, and mismatch cases. They do not require
a practical system to contain hundreds of dB of loss. Grouping tests do not
claim that stages can be reordered or that floating-point operations are exactly
associative. API agreement proves consistency; analytic pad/gain expectations
and physical properties provide separate checks on correctness.

All compared complex entries must be finite. Use absolute complex error near
zero and relative complex error when magnitude is meaningful. The practical
fixtures use a `1e-12` absolute floor and `1e-9` relative tolerance. These are
test acceptance bounds for the documented fixtures, not universal model error
budgets. The complex comparison is `abs(actual - expected) <= 1e-12 + 1e-9*abs(expected)`.
Matched gain/loss checks also require dB error below `1e-8`.
See [the test source](../tests/cascade_consistency.rs) for the fixtures.

```sh
cargo test --test cascade_consistency
just fmt-check test doc-check
```

The deep-loss cancellation investigated in [ABCD numerics](abcd-numerics.md)
remains outside this change. Shared validation prevents silent invalid outputs;
it cannot detect every finite but inaccurate result of cancellation.
