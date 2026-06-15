# Maxima Rust Kernel v9.0 — Completion & Hardening

**Status: ✅ All sprints (V1–V4) complete and merged.**

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
| **V2** | Euler substitution `∫1/((x+r)√Q)` (v8.0 S3 Family D) — branch-restricted verify | Medium | ✅ Done |
| **V3** | Puiseux (fractional-exponent) series — v8.0 S4 deferral | Medium | ✅ Done |
| **V4** | Risch RDE rational-`B` case (`∫x·exp(x)/(x+1)²`) — v8.0 S2 deferral | Medium | ✅ Done |

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
- **V2** — re-implemented the Euler substitution `∫1/((x+r)√Q)` (deferred in
  v8.0 S3). Root cause of the v8.0 failure: the result has a **branch cut** at
  `x=-r` (the `sign(u)` term), so the generic both-sides verifier rejected
  both signs. Fix: verify on the single branch `x > -r` (`verify_on_branch`).
  Also handles the degenerate case where `r` is a root of `Q` (then `P(u)` is
  linear → `√(linear)`). Results: `∫1/(x√(x²+1)) = -asinh(1/x)`,
  `∫1/((x+1)√(x²+1))`, `∫1/((x-2)√(x²-4))`, etc.
- **V3** — Puiseux series about 0 for `f = x^q·g(x)` (q rational from
  `sqrt(x)`/`x^(p/q)` factors, g analytic): `taylor(sqrt(x)*cos(x)) =
  x^(1/2)-x^(5/2)/2+…`. Tried before the ordinary method so an explicit
  fractional power isn't swallowed (a zero factor like `sin(0)` had masked the
  `x^(neg)` blowup, giving a spurious `0` for `x^(1/3)*sin(x)` → now `x^(4/3)`).
  Composition cases (`cos(√x)`) remain deferred.
- **V4** — Risch DE with a rational solution: `∫(P/Q)·exp(c·x)` via the ansatz
  `B=M/Q`, reducing to the linear identity `M'Q − MQ' + cMQ = PQ` (exact
  Gaussian solve over Q). `∫x·exp(x)/(x+1)² = exp(x)/(x+1)`,
  `∫(x-1)·exp(x)/x² = exp(x)/x`. Nonelementary cases (`exp(x)/(x+1)²` → Ei)
  correctly stay noun. Limited to `exp(c·x)` with `c` constant; general
  exponential towers remain future work.
