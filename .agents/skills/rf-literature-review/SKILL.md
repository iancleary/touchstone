---
name: rf-literature-review
description: Refresh Touchstone's literature-backed improvement shortlist against current RF APIs, numerical contracts, and primary-source evidence. Use for research prioritization, not implementation or release execution.
---

# Turn RF literature into a bounded production proposal

## Prerequisites and boundaries

- Work in the Touchstone repository and follow its current `AGENTS.md`.
- Inspect Git status and preserve unrelated work.
- Read `docs/literature-roadmap.md` and `docs/support.md` as the existing research
  index and implemented contract, respectively. Research proposals are not APIs.
- Public source retrieval is appropriate for a literature request; do not use
  paid APIs, upload measurements, install dependencies, or publish findings
  externally without separate authorization.

## Sequence

1. Identify the user workflow: measured-file analysis, reference transformation,
   differential analysis, network connection, or broadband modeling. Keep scalar
   unit conversion and block-level gain lineups out of this crate's roadmap.
2. Inspect the actual implementation and existing public integration coverage.
   Use `rg` to find symbols and bounded reads to check behavior. Record the Git
   baseline; do not recommend an already implemented fix as a new contribution.
3. Find the primary publication and verify title, authors, year, and DOI against
   publisher or bibliographic metadata. Check search-result titles: a plausible
   first result can be a different paper. Do not invent or guess missing DOIs.
4. Retrieve an open author/institution copy when available. Inspect only relevant
   sections. Use an available PDF-reading skill, or already installed tooling,
   without implicitly installing a parser. Cite physical pages/sections and note
   partial extraction, unread equations, paywalls, and metadata-only evidence.
5. Separate the publication's claim, the current code gap, and the proposed
   adaptation. Write one bounded shipping slice per candidate, including:
   - observable user outcome and natural public API location;
   - wave normalization, real/complex reference, grid, and port-order assumptions;
   - numerical failure behavior, dependency cost, and scope exclusions;
   - an independent acceptance oracle and a concrete end-to-end workflow.
6. Rank by current user value, numerical risk, and implementation scope. Treat
   timelines and performance benefits as unmeasured until demonstrated. Keep
   experimental modeling separate from default parsing/interpolation behavior.
7. Extend `docs/literature-roadmap.md`, rather than creating a parallel research
   wiki. Ensure it remains discoverable from README. Clearly label proposals and
   distinguish a sampled diagnostic from a continuous-frequency certificate.

## Verification and common failures

- Check the diff, local documentation links, and source identities. Independently
  calculate simple numerical examples using existing tools; run Python through
  `uv run`, without adding a permanent test suite for a research-only change.
- If baseline behavior matters, run the relevant existing integration workflows
  listed in the research page. Report them as baseline checks, never proof that
  a proposed feature works.
- Two mutually consistent implementations or a round trip can share the same
  error. Prefer analytical circuits, exact rational cases, or independent
  high-precision references for future numerical implementations.
- Stable poles, sampled passivity, causal models, and trustworthy out-of-band
  predictions are different claims. Do not collapse them into “physically valid.”
- On HTTP errors or broken PDF extraction, try a legitimate alternate source or
  narrow the evidence claim. Do not bypass access controls or cite a failed
  extraction as a complete reading. Downloadable reference code still needs a
  license review before reuse.

## Completion and recovery

Report the best next feature, supporting sources, implementation boundaries, and
remaining uncertainty. A research task ends with documentation and recommendations;
it does not authorize runtime changes, commits, pushes, or releases. If an idea is
already implemented or its evidence fails, remove or demote that recommendation
while preserving the distinction between historical research and current support.
