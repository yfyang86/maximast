# Maxima Rust Kernel v13 — Gap-closing program (4 bundles)

Driven by `research/survey/FUTURE_SPRINTS.md`. Four bundles, in order. Same
discipline: **compute → verify → return; correct-or-noun, never wrong.**

## Bundle 1 — Trust & polish (Tier 0 correctness) 🚧

| # | Fix | Status |
|---|-----|--------|
| 0d | `(-1)^(2n)` printed as `-1^(2n)` — parenthesize negative/rational bases | ✅ |
| 0e | expand-before-integrate; `∫x^n = x^(n+1)/(n+1)` (n≠−1) | 📋 |
| 0g | numeric `fib`/`lucas`; exact `rank` (not f64); square-free Sturm | 📋 |
| 0a | parametric/symbolic `linsolve` (was `[x=0,y=0]`) | ✅ |
| 0b | infinite sums: convergent geometric exact, rest noun (was substituting `inf`) | ✅ |
| 0c | definite-integral `inf`-leak gating (→ noun) | ✅ |
| 0f | iterated-squaring timeout — **re-scoped**: real cost in `expand` (4097-term poly²); needs fast poly-expand / hash-consing (→ infra 1e), not the simplify flag (ineffective + flag unreliable) | ⏭️ |
| 0h | plugin name resolution ✅; parser `,numer` panic → deferred (Result-based parser refactor; ev-modifier is a feature) | ◑ |

## Bundle 2 — Solve & numbers

1a cubic/quartic radical solve + `RootOf` · 1b exact real-root isolation ·
1c arbitrary-precision bigfloat backend · 3a matrix decompositions · 3b general
eigen · 3c special-function numeric eval · 3d numeric solvers/quadrature/ODE.

## Bundle 3 — Summation completion

2a order-≥2 Zeilberger (proven certificate) · 2b harmonic/Karr–Schneider sums ·
3k generating functions / holonomic→GF.

## Bundle 4 — Analysis

1d inverse Laplace (residues) · 2e contour/residue definite integrals ·
3e Fourier transforms · 3f Frobenius/Euler ODE · 3g `desolve`/ODE systems.

## Progress notes

- **Bundle 1a** ✅ (PR): 0d negative/rational power-base parens; 0e expand-before-
  integrate (polynomial-gated) + symbolic `∫x^n`; 0g numeric `fib`/`lucas`
  (`find_recurrence(fib(n))=[-1,-1,1]`). Bundle 1 cheap items done. Remaining/deferred: 0f (→ infra 1e fast poly-expand/
hash-consing), parser robustness (Result-based parser), 0i (gruntz limit bugs),
0j (`1/(1/2)` simplify). Bundle 1 essentially done. Next: Bundle 2 (Solve & numbers).
- **0i** ◑ (partial) limit pre-pass fixes: positive *rational* leading
  coefficient was misread as negative (`limit((x^2+x)/2,x,inf)` gave minf →
  now inf) via `coeff_positive`; rational `ndeg>ddeg` now carries the
  leading-ratio sign (`-x^3/(x+1)→minf`). The remaining cases are Gruntz
  omega-rewrite-internals bugs — `1-1/(x+1)→0`, `2-3/(x^2+1)→-1`,
  `2-(1/2)^x→0` — deferred to a **limit-engine hardening** task (patching
  Gruntz internals risks the many working exp/log limits).
- **0j** ✅ `simplify_power` folds `(n/d)^e` for `|e|>=1` (was `>=2`), so the
  reciprocal `(1/2)^(-1)=2`, `1/(2/3)=3/2`, `3/(1/2)=6`. Also cleaned up the
  symbolic `linsolve` fraction forms as a side benefit.
- **0f** ⏭️ the `simplified`-flag early-return is ineffective (timeout is in
  `expand`'s 4097-term squaring, not simplify recursion) AND unsafe (flag not
  perfectly reliable — broke an integrate test); reverted. Real fix = route
  polynomial `expand` through the poly crate / hash-consing (infra).
- **0h** ◑ `resolve_plugin_path` now finds `libmaxima_<name>.<ext>` in
  target/{release,debug} + search dirs: `load_plugin("specfun")` works
  (`bessel_j(0,1.0)=0.7652`), `load_plugin("orthopoly")` works. Parser
  `panic!`-on-bad-input (incl. `,numer` ev-modifier) deferred to a Result-based
  parser task.
- **0c** ✅ improper integrals no longer leak `inf`: any infinite-bound
  candidate still containing inf/minf/und (failed limit, e.g. unresolved
  `atan(inf/√2)`) → noun; a 4-arg definite that falls through returns the
  definite noun, not the indefinite antiderivative. Working cases (`%pi`,
  `√π/2`, …) unchanged. (Proper rational-improper evaluation = Bundle 4 / 2e
  contour engine.)
- **0b** ✅ `eval_sum` infinite-bound gate: convergent numeric geometric (ratio
  by exact sampling, |r|<1) → exact value; divergent/non-geometric/symbolic →
  noun (was substituting `inf` → garbage `1-1/(1+inf)`, `inf*(1+inf)/2`).
- **Found en route (new Tier-0 follow-ups):** `gruntz_limit` wrong on
  `limit(2-(1/2)^x,x,inf)`→0, `limit(x*(x+1)/2,x,inf)`→minf (0i); `1/(1/2)`
  doesn't simplify to 2 — reciprocal of a rational (0j). Both deferred; they're
  why 0b uses exact sampling rather than the limit engine.
- **0a** ✅ exact symbolic Gauss–Jordan in `eval_linsolve` (was f64,
  `to_f64(e).unwrap_or(0.0)` zeroed symbolic RHS → `[x=0,y=0]`). Now correct;
  singular→noun. (`solve(a*x=b)` symbolic-linear + fuller ratsimp deferred.)
