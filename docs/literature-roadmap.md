# Literature-backed opportunities for Touchstone

## Recommendation and scope

This is a research shortlist, **not the implemented capability contract**. The
baseline is commit `43916b1` (v0.16.0); see [supported capabilities](support.md)
for what works today. Proposed API names below are illustrative, not promises.
No runtime behavior or dependencies were changed for this investigation.

Prioritize a trustworthy numerical foundation, then two bounded user features:
**sampled passivity diagnostics** and **real-reference renormalization**.
Mixed-mode analysis and N-port connections are natural next expansions. Vector
fitting is the most ambitious opportunity, but should be an explicit modeling
subsystem rather than a replacement for ordinary interpolation.

| Priority | Opportunity | User outcome | Relative scope and main risk |
| --- | --- | --- | --- |
| 1 | Factorized solves and numerical diagnostics | Understand when a finite conversion is nevertheless unreliable | Medium; tolerance compatibility and trustworthy condition estimates |
| 2 | Sampled passivity / symmetry report | Detect suspect passive-component measurements before composing them | Small for a bounded 1/2-port version; medium for N-port SVD support |
| 3 | Real-reference renormalization | Analyze or connect measurements taken in different reference environments | Medium; wave normalization and metadata must change together |
| 4 | Explicit mixed-mode views | Inspect differential transmission and differential/common-mode conversion | Medium; port polarity, ordering, and file semantics |
| 5 | General N-port interconnection | Compose measured splitters, couplers, fixtures, and terminations | Medium–large; internal resonances and surviving-port identity |
| 6 | Shared-pole vector-fitted models | Build compact broadband models and evaluate between measured samples | Large; model selection, delays, passivity, and solver dependencies |

These are engineering judgments, not effort estimates or measured speedups.
The priorities reflect current code gaps, not a systematic survey of all RF
literature or evidence of user demand.

## Mental model: data, transformations, and models are different things

A `Network` contains sampled complex matrices plus a frequency grid, reference
impedances, and parser diagnostics. Changing reference impedance or wave basis
changes the *meaning of the matrix entries*, not just their labels. Connecting
ports eliminates internal wave variables. Fitting a model introduces assumptions
between samples and outside the measured band.

Keep those operations separate:

- Parsing should preserve measured values and warnings, including active devices.
- Analysis should return reports without silently repairing the data.
- Transformations should produce new values with coherent metadata.
- Models should carry their fitting domain, residuals, and assumptions explicitly.

This separation prevents a parser success from being mistaken for physical
validation, and a successful numerical operation from being mistaken for an
accuracy certificate.

## 1. Factorized solves with conditioning and backward-error diagnostics

**Literature.** Higham's *Accuracy and Stability of Numerical Algorithms*, second
edition (2002), provides the numerical foundation [1]. The openly accessible
LAPACK error-analysis documentation distinguishes backward error, forward error,
conditioning, and equilibration [2]. Higham's shorter author explanation directly
warns against forming an inverse merely to solve a system [3].

**Current gap.** The four S/Y/Z conversion paths in
[`src/lib.rs`](../src/lib.rs) call `invert_matrix` and multiply by its result.
The implementation is pivoted Gauss–Jordan elimination with an absolute pivot
cutoff. It validates finite outputs, but exposes no condition estimate or error
assessment. The [cascade investigation](abcd-numerics.md) already explains why a
small residual or a finite answer is insufficient near resonance.

**Production slice.** Introduce a reusable, pivoted complex linear solve with
multiple right-hand sides. Express conversions as solves rather than explicit
inversion. For a right-sided expression `X = N D^-1`, solve `X D = N`; do not
accidentally reverse matrix order or substitute a conjugate transpose for the
ordinary transpose in a transposed implementation.

Offer an opt-in result/report containing a reciprocal-condition estimate and a
scaled residual/backward-error measure. Equilibration and iterative refinement
are candidates after an initial measured comparison, not prerequisites to add
blindly. Preserve existing failure behavior until the tolerance policy has a
separate compatibility review; an absolute `1e-12` cutoff is not an accuracy
budget. In particular, do not remove the two-port cascade denominator guard just
because a matrix solver has been introduced.

**Ship gate.** Exercise public S/Y/Z APIs against analytical loads and independently
computed high-precision matrices, including ill-conditioned and scaled cases.
Require useful diagnostics on deliberately ill-conditioned inputs. Use relative
or dB error for tiny transmission and absolute error near reflection nulls.
A round trip alone is not an oracle: two operations can share an error.
Benchmark real 2/8/32-port workflows before making performance claims. Dense
factorization has cubic cost in rank and quadratic matrix storage.

**Dependency decision.** The crate currently has no general linear-algebra
backend. Compare a narrowly scoped implementation with an optional pure-Rust
backend; do not introduce a mandatory native BLAS installation without need.
A condition estimator is additional numerical work, not a label to attach to a
pivot magnitude.

## 2. Sampled passivity and symmetry diagnostics, not automatic repair

**Literature.** Kurokawa's *Power Waves and the Scattering Matrix* (1965) establishes
the power-wave framework [4]. Marks and Williams (1992), section 3.4, explicitly
give `I - SᴴS` positive semidefinite for passive circuits with real reference
impedances, and distinguish the more general complex-reference case [5].

**Current gap.** Checked constructors establish shape and finite values, not
passivity. That is appropriate: amplifier measurements are valid inputs. There
is no public physical-quality report for users expecting a passive fixture.

**Production slice.** Add a non-mutating, opt-in report over `iter_points`, initially
for the existing real-reference, S-parameter contract. Report the largest singular
value `sigma_max(S)`, the worst sampled frequency, and the amount above one using
an explicitly configured tolerance. Treat values near the boundary as numerically
uncertain rather than silently clipping them. A bounded 1/2-port release is
reasonable if a reliable N-port SVD is not yet available. An unconverged power
iteration is not a passivity certificate: it can underestimate the maximum.

Also report a **symmetry residual** such as `max |Sij - Sji|`. Explain the
conventional reciprocal-network assumptions rather than calling this a universal
electromagnetic reciprocity test; Marks and Williams document exceptions for more
general waveguide normalizations. Never force measured data to be symmetric.

**Why this beats checking each trace.** For
`S = [[0.6, 0.6], [0.6, 0.6]]`, every individual entry is below one, yet
`sigma_max = 1.2`. Equal, coherent excitation at the two ports produces an output
power ratio of `1.44`. Elementwise magnitude checks miss this violation.

**Useful existing property.** Exact RI linear interpolation between two
contractive S matrices remains contractive: the spectral norm of their convex
combination is at most the corresponding combination of endpoint norms. This
supports documenting the current interpolation choice; it does not justify
changing it to independent magnitude/phase splines. The statement assumes a
consistent wave basis/reference and excludes extrapolation and rounding effects.

**Ship gate.** Demonstrate a lossless through (`sigma_max = 1`), a matched
attenuator (`sigma_max < 1`), and the coherent-excitation counterexample above
through public APIs. Preserve active-device inputs and all parser warnings.
Call the result **sampled passivity**, not broadband passivity or causality:
a finite measurement grid does not establish arbitrary physical behavior between
samples. There is no automatic repair in this first feature.

## 3. Renormalize real reference impedances without an unnecessary Z detour

**Literature.** Marks and Williams develop changes of reference impedance and
multiport transformations in sections 3.7–3.8 [5]. Kurokawa explains why the wave
convention matters [4]. Touchstone 2.1 restricts its reference values to real,
positive resistances [6, p. 11].

**Current gap.** `ReferenceImpedance` retains common or per-port metadata, but
network conversions reject per-port values and `try_cascade` requires a matching
common reference. There is no operation that actually transforms measured S
values into a different reference environment. Editing `z0` is not renormalization.

**Production slice.** Start with common positive real source and destination
references and an explicit operation such as `renormalize(new_z0)`. For this
restricted case, with `g = (new_z0 - old_z0)/(new_z0 + old_z0)`, the direct
transformation is

```text
S_new = (S_old - g I) (I - g S_old)^-1
```

Implement the right-sided solve rather than an explicit inverse. Unlike a
mandatory S→Z→S path, this form can handle an ideal open (`S = I`) without first
attempting to invert `I - S`. It can still have a singular denominator for some
inputs, so retain contextual errors and finite-result validation. Evaluate the
scalar ratio in a range-safe way for extreme finite impedances.

Update values, `z0`, and complete reference metadata coherently; preserve
frequencies and warnings. Do not automatically resample or renormalize as a side
effect of cascading. Extend to unequal per-port real references only after the
diagonal normalization and matrix ordering have independent validation. Complex
and frequency-dependent references require a larger wave-definition contract and
are not part of this first release.

**Ship gate.** A 75-ohm one-port load represented at 50 ohms has `S11 = 0.2`; after
renormalizing to 75 ohms it has `S11 = 0`. A matched 50-ohm load moved to 75 ohms
has `S11 = -0.2`. Ideal opens and shorts remain `+1` and `-1`. Include an
independent multiport circuit reference, metadata serialization, and preservation
of warnings; do not rely only on returning to the original impedance.

## 4. Mixed-mode analysis with explicit port pairing and polarity

**Literature.** Bockelman and Eisenstadt's *Combined Differential and Common-Mode
Scattering Parameters: Theory and Simulation* (1995) is the foundational RF
reference [7]. The actual file contract is Touchstone 2.1, pages 20–23 [6].

**Current gap.** N-port matrices are available, but `[Mixed-Mode Order]` is treated
as an unknown keyword. Its semantics are not implemented or preserved. The
support table correctly warns against writing such input through this API.

**Production slice.** First provide a checked mixed-mode **view/result type** of a
known single-ended network, not a `Network` with ambiguous port identities. An
explicit ordered pair `(positive_port, negative_port)` defines polarity. Validate
non-overlap, complete mode mapping, and equal real reference resistances within
each pair. For one pair in differential/common order,

```text
T = (1/sqrt(2)) [[1, -1], [1, 1]]
S_mixed = T S_single T^T
```

For multiple pairs, construct the full transformation and explicit output order;
retain unpaired single-ended ports as identity rows. The resulting physical modal
references are `2R` differential and `R/2` common: a 50-ohm pair gives 100 and
25 ohms, respectively. This is a basis transformation, not subtracting dB traces.

**Important file boundary.** Touchstone's `[Reference]` still lists the
*single-ended* reference resistances even when its data is mixed-mode. Do not
write 100/25 as though those were the original pair's `[Reference]` values.
Serialization must wait for a coherent `[Mixed-Mode Order]` model, its descriptor
validation, and preserved single-ended reference metadata. Parsing a mixed-mode
file and then applying the transform again would also be wrong.

**Ship gate.** Use a balanced four-port connection with analytically zero
common/differential conversion, then a deliberately asymmetric path with known
nonzero conversion. Check exact mode labels and polarity-sensitive complex
results, not just a basis-transform round trip. Parser support, when added,
requires named fixtures in `files/` and public integration coverage.

## 5. General N-port connections via internal-wave elimination

**Literature.** Redheffer's *On the Relation of Transmission-Line Theory to
Scattering and Transfer* (1962) is the classical scattering-composition reference
[8]. The public scikit-rf connection API is a practical implementation reference,
not a numerical oracle [9].

**Current gap.** `cascade_ports` in [`src/lib.rs`](../src/lib.rs) accepts only two
2-port networks connected port 2→1; other valid-looking requests panic. General
interconnection would make the crate's N-port parsing and matrix APIs much more
useful for measured couplers, dividers, and multiport fixtures.

**Production slice.** Add a fallible `connect` operation with an explicit port map,
equal connected-port reference impedances, aligned frequency grids, and documented
surviving-port order. Initially support a single connection between two networks,
then multiple joins and one-port terminations. Do not route through ABCD.

To see the mechanism, partition a combined S matrix into external (`e`) and
internal (`i`) ports. With an ideal internal connection `a_i = P b_i`, solve

```text
(I - P S_ii) X = P S_ie
S_effective = S_ee + S_ei X
```

Here `P` swaps the members of each connected pair. For specified load terminations,
a reflection matrix replaces the appropriate connection blocks. This derivation
shows where the internal resonance lives; the existing two-port formula is the
scalar special case. Removing ports without this calculation only describes
matched termination of those ports, not an arbitrary physical connection.

**Ship gate.** Reproduce current two-port cascade results, then show a measured or
analytic three-port splitter with one port terminated and independently calculated
remaining responses. Test external-port identity, mismatched-reference rejection,
and singular internal connections. Do not silently choose a pseudoinverse when
the internal solve is singular: that changes the solution policy. Dense solve
cost depends cubically on the number of internal wave unknowns; no sparse circuit
engine is required for the first implementation.

## 6. Vector fitting as an explicit broadband modeling subsystem

**Literature.** Gustavsen and Semlyen's *Rational Approximation of Frequency Domain
Responses by Vector Fitting* (1999) fits partial-fraction models by pole relocation,
then residue identification. It explicitly supports shared poles for vector
responses, complex-conjugate pole pairs, and stabilization [10, sections 2–3].

**Current gap.** `sample_at` and `resample` provide RI linear interpolation and
nearest-point sampling. Neither builds a compact rational model, resolves an
undersampled resonance, or provides a simulation-ready dynamic representation.

**Production slice.** Introduce a separate `RationalNetworkModel`, initially as an
experimental optional feature, with a bounded pole count and iteration count:

```text
S(s) = D + sum_k R_k / (s - p_k),    s = j 2 pi f
```

Fit shared poles across selected matrix entries; require conjugate structure for a
real time-domain realization, and stable poles for the stable-model mode. Use QR
or SVD least squares, not normal equations. Start with proper models (no `s E`
term), explicit training band, and explicit treatment of transport delay. Long
cables can otherwise consume many poles just approximating their delay.

Report held-out complex error, maximum error as well as RMS, pole/iteration
counts, and the measured band. Absolute error matters near transmission zeros;
relative error is useful elsewhere. The original paper's reported errors and
runtimes are not expected results for this crate or modern hardware.

**Ship gate.** Recover known resonator behavior on frequencies withheld from
fitting, then evaluate a real passive measurement with an independently assessed
response. Compare against the existing linear baseline using the same withheld
points. A fit cannot recover resonances absent from the measurements.

**Critical limits.** Stable poles do not guarantee passivity. A causal stable
proper fitted model does not establish that the original finite-band measurement
was causal or that its out-of-band response is known. Passivity enforcement and
all-frequency assessment require further algorithms and their own literature
review; sampled clipping is not a substitute. Do not make extrapolation the
default. Audit licenses before adapting reference code: an available paper or
historical availability statement does not establish the license of a current
implementation.

## Suggested delivery sequence and explicit deferrals

1. Establish a solver/error-policy experiment using the existing matrix and
   numerical-guarantee workflows; decide the backend from accuracy and deployment
   evidence, not algorithm fashion.
2. Ship sampled diagnostics and scalar-real renormalization as separate bounded
   features. Diagnostics need not wait for a full N-port solver if initially scoped
   to 1/2 ports. Keep parsing permissive and reports non-mutating.
3. Choose mixed-mode views or N-port connections based on an actual user's
   four-port or splitter workflow. Both are useful; neither requires vector fitting.
4. Prototype vector fitting only with a concrete model-consumption workflow and
   held-out reference data. Do not commit to automatic passivity repair or SPICE
   export until their conventions and validation are independently specified.

Defer generic de-embedding, automatic causality certification, complex-reference
support, GPU fitting, and a sparse circuit simulator. They enlarge the contract
substantially without first fixing the present numerical and metadata gaps.
Also keep parser conformance defects separate: literature-inspired features do
not replace the concrete gaps already recorded in [support.md](support.md).

## Sources and evidence quality

All six scholarly DOI records below were checked against Crossref for matching
title/authors/year (references 1, 4, 5, 7, 8, 10). Access depth is listed so metadata
verification is not confused with reading a paper. This is a targeted review, not
a claim to have read every cited work in full.

1. **N. J. Higham (2002).** *Accuracy and Stability of Numerical Algorithms*,
   second edition. [DOI: 10.1137/1.9780898718027](https://doi.org/10.1137/1.9780898718027).
   Book metadata verified; supporting accessible explanations read in [2–3].
2. **LAPACK Users' Guide.**
   [Further Details: Error Bounds for Linear Equation Solving](https://www.netlib.org/lapack/lug/node81.html).
   Open text inspected, including the roles of `RCOND`, `BERR`, `FERR`, and
   equilibration. Extracted HTML loses equations rendered as images; no numerical
   bound in this proposal is taken from that incomplete equation rendering.
3. **N. J. Higham (2022).**
   [Seven Sins of Numerical Linear Algebra](https://nhigham.com/2022/10/11/seven-sins-of-numerical-linear-algebra/).
   Author explanation inspected, especially inversion, normal equations, and
   conditioning. Supplementary explanation, not a substitute for the book.
4. **K. Kurokawa (1965).** *Power Waves and the Scattering Matrix*.
   [DOI: 10.1109/TMTT.1965.1125964](https://doi.org/10.1109/TMTT.1965.1125964).
   Metadata verified; the applicable real-reference passivity condition was read
   directly in Marks and Williams, section 3.4.
5. **R. B. Marks and D. F. Williams (1992).** *A General Waveguide Circuit Theory*.
   [DOI: 10.6028/jres.097.024](https://doi.org/10.6028/jres.097.024);
   [open NIST PDF](https://nvlpubs.nist.gov/nistpubs/jres/097/jresv97n5p533_A1b.pdf).
   Physical pages 1–3 and 15–16 inspected, including sections 3.4–3.7. Extraction
   failed on some other pages; section 3.8 is identified by the contents, not
   claimed as successfully read. General unequal-reference formulas still need
   a complete source/equation review before implementation.
6. **IBIS Open Forum (2024).** *Touchstone File Format Specification Version 2.1*.
   [Local PDF](touchstone_ver2_1.pdf);
   [published PDF](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf).
   Pages 11 and 20–23 inspected for reference resistance and mixed-mode semantics.
7. **D. E. Bockelman and W. R. Eisenstadt (1995).** *Combined Differential and
   Common-Mode Scattering Parameters: Theory and Simulation*.
   [DOI: 10.1109/22.392911](https://doi.org/10.1109/22.392911).
   Metadata verified; the proposed restricted transform is supported directly by
   the inspected IBIS definitions, not a claim to have read this paper in full.
8. **R. Redheffer (1962).** *On the Relation of Transmission-Line Theory to
   Scattering and Transfer*.
   [DOI: 10.1002/sapm19624111](https://doi.org/10.1002/sapm19624111).
   Metadata verified; publisher full text returned HTTP 403. The block elimination
   above is an explicit derivation, not a quoted equation from an unread paper.
9. **scikit-rf documentation.**
   [`connect_s`](https://scikit-rf.readthedocs.io/en/latest/api/generated/skrf.network.connect_s.html).
   Public behavior inspected: connecting one port pair produces `n + m - 2` ports.
   Software/API precedent only, not a scholarly source or accuracy oracle. Its
   port indices are 0-based; Touchstone's public RF port APIs are 1-based.
10. **B. Gustavsen and A. Semlyen (1999).** *Rational Approximation of Frequency
    Domain Responses by Vector Fitting*.
    [DOI: 10.1109/61.772353](https://doi.org/10.1109/61.772353);
    [author-hosted PDF](https://www.sintef.no/globalassets/project/vectfit/vf_paper.pdf).
    Selected text from physical pages 1–3 and 6–7 inspected for algorithm,
    conditioning, applications, and limits; not a full-paper reading.

## Revisit and verify

Use the repository skill at `.agents/skills/rf-literature-review/SKILL.md` to refresh
this research against future code changes. Start with the support table and
source, recheck primary references, then agree on one bounded implementation.
Existing useful baseline checks are:

```sh
cargo test --test matrix_api --test numerical_guarantees --test cascade_consistency
cargo test --test readme_examples
```

For this investigation, the combined command below passed all 47 existing tests:

```sh
cargo test --offline --test matrix_api --test numerical_guarantees --test cascade_consistency --test readme_examples
```

Local Markdown link targets and `git diff --check` passed. Independent arithmetic
sanity checks confirmed the one-port renormalization examples, the `1.44` power
counterexample, the mixed-mode map against voltage/current definitions, and the
block connection formula's reduction to scalar two-port composition. These are
formula/documentation checks, not implemented-feature evidence.

The integration suites exercise existing behavior; they do not validate any
proposed feature. Future implementation should follow the repository's fixture
and public-API verification rules and update the support contract only after
behavior exists.
