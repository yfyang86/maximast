# Maxima Rust Kernel v13 — Gap-closing program (4 bundles)

Driven by `research/survey/FUTURE_SPRINTS.md`. Four bundles, in order. Same
discipline: **compute → verify → return; correct-or-noun, never wrong.**

## Bundle 1 — Trust & polish (Tier 0 correctness) 🚧

| # | Fix | Status |
|---|-----|--------|
| 0d | `(-1)^(2n)` printed as `-1^(2n)` — parenthesize negative/rational bases | ✅ |
| 0e | expand-before-integrate; `∫x^n = x^(n+1)/(n+1)` (n≠−1) | 📋 |
| 0g | numeric `fib`/`lucas`; exact `rank` (not f64); square-free Sturm | 📋 |
| 0a | parametric/symbolic `linsolve` & `solve` (was `[x=0,y=0]`) | 📋 |
| 0b | infinite sums via `limit(S(m),m,inf)` (was substituting `inf`) | 📋 |
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
  (`find_recurrence(fib(n))=[-1,-1,1]`). Next: 0a/0b/0c, then 0f/0h.
