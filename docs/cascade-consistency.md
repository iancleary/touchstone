# Cascade consistency

Network cascades use checked direct scattering composition in f64. Public
S/Y/Z/ABCD conversion APIs remain available independently. The private legacy
ABCD helpers delegate to the public conversions.

## Connection and error contract

Use `Network::try_cascade` to receive `Result<Network, TouchstoneError>`.
The existing `cascade` method delegates to it and panics on error. Multiplication
and `cascade_ports` retain their signatures and use that same path. Left port 2
connects to right port 1.

Inputs must be finite two-port S-parameter networks with the same positive
common real reference impedance, frequency unit, and frequency grid. The grid
must be nonempty, nonnegative, and strictly increasing. Empty, misaligned, or
malformed data is rejected. Resample explicitly before cascading different grids.

For left network L and right network R, define `d = 1 - L22*R11`. The connected
network has:

```text
S11 = L11 + L12*R11*L21/d
S12 = L12*R12/d
S21 = R21*L21/d
S22 = R22 + R21*L22*R12/d
```

The denominator must have magnitude above `1e-12`; otherwise the operation
returns `SingularMatrix` with the frequency-point index and operation
`cascade internal connection`. This is a singularity policy, not an accuracy
bound. The method supports zero forward transmission when the internal
connection is nonsingular. It preserves distinct forward and reverse paths.

Successful cascade, sampling, and S/Y/Z/ABCD conversion results contain finite
real and imaginary entries. Nonfinite results return structured errors. Parsing
can retain invalid source values; numerical operations check values used for
computation. A zero complex magnitude can legitimately produce negative infinity
in a derived dB view. This is distinct from a nonfinite complex S parameter.

## Independent accuracy checks

| Property | Concrete example | Expected result |
| --- | --- | --- |
| Through identity | Add a through before or after a complex network | All four S entries remain unchanged within tolerance |
| Matched loss | Cascade matched 3 dB and 6 dB pads | 9 dB loss in both directions |
| Matched gain | A 20 dB amplifier followed by a 6 dB pad | 14 dB forward gain; reverse transmission follows its specified value |
| Deep loss | Two 90 dB stages; two 120 dB stages; twelve 20 dB stages | Analytic loss sums hold in both transmission directions |
| Range probe | Two 1500 dB matched stages | Both transmissions remain approximately `1e-150` |
| Nonreciprocity and mismatch | Complex entries with exact decimal inputs | Agreement with an exact rational internal-wave solution |
| Blocked forward path | Zero S21 followed or preceded by a through | Finite unchanged network; ABCD conversion still rejects zero S21 |
| Singular connection | `L22*R11` equals or lies within `5e-13` of one | Structured error |
| Physical conversion | 100-ohm series resistor and 50-ohm shunt resistor | Analytic S, Y, and Z entries |
| Grouping | Compare `(A cascade B) cascade C` with `A cascade (B cascade C)` | Same stage order agrees within tolerance |
| Extreme arithmetic | Complex division, RI magnitude, interpolation, and parameter conversions across small and large finite scales | Expected finite values where tested; every successful numerical result has finite complex entries |

Deep-loss comparisons require relative transmission error below `5e-13` and
dB error below `1e-9`. They use no absolute transmission floor, so a zero result
cannot pass for a tiny nonzero expected transmission. The complex rational
fixture uses absolute error `1e-15` plus relative error `1e-13`. These are
acceptance bounds for the named fixtures, not universal accuracy guarantees.
See [numerical guarantees](../tests/numerical_guarantees.rs) and
[cascade consistency tests](../tests/cascade_consistency.rs).

The ordinary fixtures also compare direct cascade with public ABCD composition.
That comparison is a consistency check in a well-conditioned range. ABCD-to-S
still reconstructs reverse transmission through `A*D-B*C`; it can lose accuracy
for deep loss. Its existing `1e-12` cutoffs apply to `2*S21` and
`A+B/z0+C*z0+D`. They do not apply to the direct cascade's forward transmission.

Finite validation does not detect every finite inaccurate result. Internal
resonances, matrix inversion conditioning, extreme intermediate overflow, and
f64 underflow remain limits. An operation can return an error even when an
exact-arithmetic answer is finite. There is no automatic clamp, passivity
check, forced reciprocity, or higher-precision fallback.

```sh
cargo test --test numerical_guarantees --test cascade_consistency
cargo test --example abcd_numerics
just fmt-check test doc-check
```

The [ABCD investigation](abcd-numerics.md) preserves the historical failure
measurements and explains the choice of direct scattering composition.
