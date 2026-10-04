# Supported capabilities

This document describes the implemented contract. It does not claim full
Touchstone standard conformance. The reference is the
[IBIS Touchstone 2.1 specification](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf).

## Operations

| Operation | Supported | Limits |
| --- | --- | --- |
| Parse | Files, UTF-8 strings, and byte slices; full N-port matrices; RI, MA, and DB | Use single-ended S-parameter data. Vendor fixtures cover 1 through 32 ports. An accepted file is not proof of full standard conformance. |
| Write | String, generic `Write`, and file destinations | Writes the supported Touchstone 2.1 full-matrix subset. It does not preserve the source version, whitespace, or original line layout. |
| Plot | Interactive HTML for one-port and two-port S networks | All networks in one plot must have the same rank. N-port plotting is unavailable above two ports. Directory discovery currently handles only single-digit `.sNp` extensions. |
| Construct | `NetworkBuilder`; checked matrix and point constructors | Generated networks require a nonempty, finite, nonnegative, strictly increasing frequency grid and finite square S matrices of the declared rank. DC is valid. |
| Read without copying | `frequencies`, `point_ref`, `iter_points` | Borrowed views expose values without mutable parser storage. Frequency-point indexes are 0-based; RF port indexes are 1-based. |
| Sample and resample | Linear interpolation of real/imaginary components, or nearest point; error or clamp extrapolation | Source and output grids must be nonempty, finite, nonnegative, and strictly increasing. Parsed grids that violate these rules can be inspected but cannot be sampled. |
| Convert S/Y/Z | Full square matrices with a common positive, finite, real reference impedance | Singular or non-finite calculations return errors. Network-level conversions reject non-S source data and per-port reference impedances. |
| Convert S/ABCD | Two-port matrices with a common real reference impedance | ABCD conversion requires nonzero forward transmission and applies a `1e-12` denominator cutoff. Large attenuation can lose information in the ABCD representation. |
| Cascade | Two-port networks with matching frequency grids and a common reference impedance | Uses direct S composition. Zero forward transmission is valid when the internal connection is solvable. A near-singular internal connection returns an error. Display frequency units must currently also match. |

Numerical results returned by successful conversions, sampling, and cascading
have finite complex entries. A zero S-parameter has a valid dB representation
of negative infinity. This is distinct from a non-finite complex result.
See [numerical acceptance tests and limits](cascade-consistency.md).

## File versions and matrix representation

| Construct | Read behavior | Write behavior |
| --- | --- | --- |
| Legacy version 1.x layout without `[Version]` | Reads the supported single-ended S subset; infers rank from `.sNp` | Emits version 2.1 |
| Explicit `[Version] 2.0` or `2.1` | Accepted for the implemented keywords below | Emits version 2.1 |
| Other explicit versions | Rejected | Not emitted |
| `.sNp` extension | Lowercase `s`/`p` required for rank inference; rank must agree with `[Number of Ports]` when present | Caller chooses the output path |
| `.ts` extension | Unsupported, even with `[Number of Ports]` | No filename-based conversion |
| `[Matrix Format] Full` | Supported | Always emits `Full` |
| `Lower` or `Upper` matrices | Rejected | Unsupported |
| Two-port order `21_12` or `12_21` | Both supported; legacy default is `21_12` | Emits `21_12` |
| RI, MA, DB | Supported; angles are degrees | Preserves the network's format setting; builder defaults to RI |
| Hz, kHz, MHz, GHz | Converted to Hz on input | Converts stored Hz to the selected output unit |
| THz | Not correctly recognized by the option-line parser; do not use as an input unit | Builder/writer can emit THz as an extension, but it is not a supported round trip |
| Y, Z, H, G file parameters | Tokens and values can be parsed, but S accessors do not convert their meaning | Unsupported semantic round trip, including legacy normalization differences |
| Mixed-mode data | `[Mixed-Mode Order]` is treated as an unknown keyword; its meaning is not implemented | Declaration is not preserved; do not save mixed-mode input through this API |
| Noise records | Unsupported; bundled noise fixtures return errors | Not emitted |

## Metadata and diagnostics

| Metadata | Behavior |
| --- | --- |
| Scalar option-line `R` | Positive finite real impedance; default is 50 ohms |
| `[Reference]` | A common value or one positive finite real value per port; takes precedence over scalar `R` |
| `[Reference]` continuation | Values may occupy the keyword line or one following line. General multiline lists are unsupported. |
| Per-port `R` values on the option line | Unsupported; current token parsing can collapse them to the last value. Use `[Reference]` within the supported v2 subset. |
| Full-line comments | Retained in the before-option and after-option comment collections; placement is normalized during writing |
| Inline data comments | Ignored when reading numerical values; not preserved |
| Inline option-line comments | Known parser gap: comment tokens can change options or cause errors. Remove these comments before parsing. |
| Missing option line | Uses defaults and records `MissingOptionLine` |
| Additional option lines | Ignored after the first; each produces a warning |
| Unknown keywords | Produce warnings; their values and meaning are not preserved. Inspect warnings before numerical use or writing. |
| Number of ports/frequencies | Checked when declared; the writer emits these counts |
| Vendor gamma/Z0 annotations | Preserved only as comments where applicable; not interpreted as frequency-dependent complex reference impedances |
| Invalid numeric grids | Parsing can retain duplicate, descending, negative, or non-finite frequencies. Construction and sampling enforce the stricter grid contract above. |

## API compatibility

`Network.s` and `Network.f` are private. Use the trace and matrix read APIs or
borrowed views. Use `NetworkBuilder` to construct a generated network.
`f()` still returns an owned frequency copy.

`SMatrix::try_new`, `ParameterMatrix::try_new`, `ABCDMatrix::try_new`, and
`NetworkPoint::try_new` validate their inputs. The existing owned value types
remain editable. Numerical operations and `NetworkBuilder::build` validate them
again at use boundaries.

The infallible `cascade` and multiplication convenience APIs retain their panic
contract. Use `try_cascade` to handle invalid connections. The CLI uses this
fallible path for user-supplied cascade inputs.
