# v8.0 Sprint Index

## Status: 📋 Planned

## Theme

**Deepen the symbolic core** — complete the differential / difference-algebra
layer (integration & summation tower). This is the project's strategic
differentiator: the place Maxima is weakest, FriCAS leads, and no Rust
competitor exists. Direct continuation of `research/survey/ALGORITHM_SURVEY.md`.

## Sprints

| Sprint | Content | Size | Status |
|--------|---------|------|--------|
| **S1** | Lazard–Rioboo–Trager logarithmic part (rational integration spine) | Medium | 📋 |
| **S2** | Complete Risch differential-equation solver (decision-complete transcendental) | Large | 📋 |
| **S3** | General Euler substitution for `∫R(x,√(ax²+bx+c))` (the removed gap) | Medium | 📋 |
| **S4** | Robust power-series engine (Laurent / Puiseux) | Medium | 📋 |
| **S5** | Holonomic closure + Almkvist–Zeilberger (definite integrals) | Large | 📋 |
| **S6** | Trager radical-only algebraic integration | Large | 📋 |
| **S7** | Named nonelementary antiderivatives (li, Ei, erf/erfi, Si, Ci, Fresnel) | Small | 📋 |

## Phases

| Phase | Sprints | Focus |
|-------|---------|-------|
| **Phase 1 — Rational+Transcendental** | S7 → S1 → S2 | Make the existing tower correct and decision-complete |
| **Phase 2 — Algebraic frontier** | S3 → S6 | The repeatedly-deferred √-integration gap |
| **Phase 3 — Definite & series depth** | S4 → S5 | Reuse Zeilberger/Ore infra for integrals |

## Key Metrics (target)

| Metric | Before | After |
|--------|--------|-------|
| Tests | 1117 | ~1180+ |
| Walkthroughs | 41 | ~43 |
| Headline new capability | — | erf/li/Ei antiderivatives, Euler √-integration, general definite integrals via AZ |

## Documents

| File | Contents |
|------|----------|
| [PLAN.md](PLAN.md) | Full sprint plan: tasks, algorithms, verification, risks, open questions |

## Explicitly Deferred

Meijer-G tables · Karr/Schneider ΠΣ summation · Risch parametric (Raab) ·
CAD / quantifier elimination · multivariate polynomial factoring.
See PLAN.md "Out of Scope" for rationale.
