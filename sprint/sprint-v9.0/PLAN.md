# Maxima Rust Kernel v9.0 — Completion & Hardening

## Theme

Close out the items deferred during v8.0 and fix bugs surfaced along the way.
v8.0 ("deepen the symbolic core") landed the integration/series tower; v9.0
hardens it — correctness fixes plus the achievable deferrals — before the next
big strategic push (multivariate polynomials / Reduce-CAD / full A–Z).

Same discipline as v8.0: every returned closed form passes the numeric
differentiate-back gate; noun form beats a wrong answer; each sprint is its own
PR to `dev`, merged when green.

## Sprints

| Sprint | Content | Size | Status |
|--------|---------|------|--------|
| **V1** | Fix `∫1/√(ax²+bx+c)` leading-coefficient bug; clean inverse-fn arg display | Small | ✅ Done |
| **V2** | Euler substitution `∫1/((x+r)√Q)` (v8.0 S3 Family D) — fix verify/simplify interaction | Medium | 📋 |
| **V3** | Puiseux (fractional-exponent) series — v8.0 S4 deferral | Medium | 📋 |
| **V4** | Risch RDE rational-`B` case (`∫x·exp(x)/(x+1)²`) — v8.0 S2 deferral | Medium | 📋 |

## Carried-forward backlog (beyond v9.0)

Full Almkvist–Zeilberger / holonomic engine · full Trager for algebraic
function fields · Meijer-G · Karr ΠΣ summation · multivariate polynomial
engine · Reduce/CAD. See `sprint/sprint-v8.0/SPRINT_INDEX.md`.

## Progress notes

- **V1** — `∫1/√(4x²+1)` returned `log(x+√(1+x²))` (a table handler built the
  result as if the leading coefficient were 1). Guarded the `#72`/`#81` monic
  arms to `a==1` so non-monic quadratics fall through to the verified
  quadratic-radical path (`∫1/√(4x²+1) → asinh(2x)/2`). Also `rat_eval`'d the
  asinh/asin arguments for clean output (`asinh(2*x)` not `asinh(8*x/4)`).
