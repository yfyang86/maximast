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
| 0c | definite-integral `inf`-leak gating | 📋 |
| 0f | `simplify` honors the `simplified` flag (iterated-squaring timeout) | 📋 |
| 0h | plugin name resolution; `,numer`/`,modulus` ev-modifier parse | 📋 |

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
  (`find_recurrence(fib(n))=[-1,-1,1]`). Next: 0c, then 0f/0h.
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
