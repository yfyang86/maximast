# v8.0 Sprint Index

## Status: 🚧 Phase 1 complete (S7, S1, S2); S3 done (A/B/C, Euler deferred)

## Theme

**Deepen the symbolic core** — complete the differential / difference-algebra
layer (integration & summation tower). This is the project's strategic
differentiator: the place Maxima is weakest, FriCAS leads, and no Rust
competitor exists. Direct continuation of `research/survey/ALGORITHM_SURVEY.md`.

## Sprints

| Sprint | Content | Size | Status |
|--------|---------|------|--------|
| **S1** | Lazard–Rioboo–Trager logarithmic part (rational integration spine) | Medium | ✅ Done |
| **S2** | Risch DE fix + named nonelementary antiderivatives (pragmatic scope) | Large | ✅ Done |
| **S3** | Quadratic-radical integrals `∫R(x,√(ax²+bx+c))` (complete-the-square A/B/C; Euler deferred) | Medium | ✅ Done |
| **S4** | Robust power-series engine (Laurent + reduced coeffs; Puiseux deferred) | Medium | ✅ Done |
| **S5** | Holonomic closure + Almkvist–Zeilberger (definite integrals) | Large | 📋 |
| **S6** | Trager radical-only algebraic integration | Large | 📋 |
| **S7** | Named nonelementary antiderivatives (erf/erfi, expintegral_*, fresnel_*) | Small | ✅ Done |

## Progress notes

- **S7** — nine special functions as built-ins (Maxima names `erf`/`erfc`/`erfi`/`expintegral_*`/`fresnel_*`) with diff/float/help.
- **S1** — `lazard_rioboo_trager` wired into rational integration as a verified, no-regression log-part method; clean logs like `∫(5x⁴+1)/(x⁵+x) → log(x⁵+x)`.
- **S2** — pragmatic Risch scope: fixed a wrong-answer substitution bug (`∫x·exp(x²)` now `exp(x²)/2`) and added verified named results (`∫exp(x²) → erfi`, `∫1/log(x) → li`, `∫exp(x)/x → Ei`, `∫sin(x)/x → Si`, `∫cos(x)/x → Ci`). Rational-`B` RDE (e.g. `x·exp(x)/(x+1)²`) deferred.
- **S3** — quadratic-radical integrals via completing the square, all gated by a numeric differentiate-back verifier: Family A `∫1/√(ax²+bx+c)` (asinh/asin/log), Family B `∫(px+q)/√Q`, Family C `∫√Q dx`. New cases like `∫√(x²-1)`, `∫(2x+3)/√(x²+1)`, `∫x/√(2x-x²)`. **Deferred:** Family D `∫1/((x+r)√Q)` (Euler `u=1/(x+r)`) — the candidate is correct but a verify/simplify interaction rejects it; returns noun (never a wrong answer).

- **S4** — power-series robustness. Rewrote `taylor` to (1) reduce coefficients via `meval` (`x^3/3` not `2*x^3/6`) and (2) compute **Laurent series** by series-dividing the numerator/denominator Taylor coefficients with pole extraction — fixing the previously-broken `und` output: `1/(exp(x)-1) → 1/x-1/2+x/12-x³/720`, `1/sin(x) → 1/x+x/6+7x³/360`, `cos(x)/x → 1/x-x/2+x³/24`. Series-backed limits unchanged. **Deferred:** Puiseux (fractional-exponent) series.

## Follow-ups discovered

- **Pre-existing bug (not S3):** `∫1/√(4x²+1)` returns `log(x+√(1+x²))` — a stale handler ignores the leading coefficient 4 (wrong). Fix by letting the verified quadratic-radical path take precedence, or correcting the old handler.
- Display polish: quadratic-radical results are correct but unreduced (`3*asinh(2*x/2)`, `2*x*√(…)/4`).

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
