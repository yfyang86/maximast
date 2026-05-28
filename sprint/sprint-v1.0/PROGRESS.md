# Sprint Status

## v1.0 RC Completion (Frozen at v1.0 branch)

| RC | Status | Maxima Coverage |
|----|--------|----------------|
| **RC0** | **Complete** | Expr types, parser, REPL |
| **RC1** | **Complete** | Full parser (100%), evaluator (145+ built-ins) |
| **RC2** | **Complete** | Simplifier (~23% of Maxima's depth) |
| **RC3** | **Complete** | Assumptions (~25% of Maxima's compar.lisp) |
| **RC4** | **Complete** | Polynomials (~30% of Maxima's rat/factor) |
| **RC5** | **Complete** | Calculus (~10% of Maxima's limit/integration depth) |
| **RC6** | **Complete** | Solving, matrices, tex, REPL UX |

## v2.0 Algorithmic Foundations (Current — 2026-05-25)

All 8 core sprints (S1-S8) complete. See `sprint-v2.0/REVISED_PLAN.md`.

| Module | v1.0 | v2.0 | Coverage | Notes |
|--------|------|------|----------|-------|
| Parser | 967 | 967 | **100%** | Full Maxima syntax |
| Evaluator | 5,581 | 7,800+ | **100%** | 150+ built-ins, sum/product |
| Simplifier | 751 | 751 | **~25%** | Pythagorean, De Morgan added |
| Assumptions | 705 | 705 | **~25%** | Sign, transitive |
| Limits | 207 | 600 | **~15%** | MRV Gruntz, series, 0*∞ |
| Def. Integration | ~100 | 300+ | **~10%** | Infinite bounds, residues, Gaussian |
| Risch Integration | 0 | 800+ | **~30%** | Tower, primitive/exp case, substitution |
| Summation | ~50 | 350+ | **~25%** | Faulhaber, Gosper, telescoping |
| Poly/GCD | 951 | 1,100+ | **~35%** | GCD fix, CRE, series type |
| Solving | ~200 | ~200 | **~22%** | Poly roots via factoring |
| Display (2D) | 0 | 0 | **0%** | Not implemented |

### Test Counts
| Version | Total Tests | eval | poly | parser | core |
|---------|------------|------|------|--------|------|
| v1.0 | 326 | 180 | 95 | 78 | 43 |
| v2.0 | **644** | **404** | **106** | 78 | 43 |
| TeX | 1,262 | 147 | **~12%** | Basic output |
| Plot | 2,708 | 0 | **0%** | Not implemented |
| Matrix | 683 | ~200 | **~29%** | Basic ops + eigen |
| **Total src/** | **135,125** | **12,899** | **~10%** | |

## Not Covered (Out of Scope)

| Category | Maxima Lines | Status |
|----------|-------------|--------|
| Special functions (gamma, bessel, elliptic) | ~31,000 | Not implemented |
| Numerical (SLATEC/QUADPACK) | ~28,500 | Not implemented |
| Share packages (66 packages) | ~100,000+ | Not implemented |
| Plot subsystem | ~2,700 | Not implemented |
| GUI (xmaxima, emacs) | ~10,000+ | Not implemented |

## rtest Pass Rates

| File | Rate |
|------|------|
| rtest1 | **79%** (164/208) |
| rtest_boolean | **76%** (88/116) |
| rtest_equal | **54%** (112/208) |
| rtest_everysome | **54%** (45/84) |
| rtest_algebraic | **53%** (24/45) |
| rtest_abs | **50%** (71/141) |
| rtest_dot | **50%** (36/72) |
| **Files ≥ 50%** | **7 of 99** |

## What the Rust Kernel CAN Do

- Parse and evaluate all Maxima syntax
- Simplify algebraic expressions (canonical ordering, term collection, boolean)
- Differentiate all elementary functions (chain rule, nth derivative)
- Integrate ~50 patterns (polynomials, trig, exp, log, by-parts, rational, completing square)
- Compute limits (finite, infinite, L'Hôpital, Gruntz for exp/log)
- Factor polynomials (square-free + Kronecker quadratic search)
- Compute polynomial GCD, extended GCD, resultants
- Solve equations (any degree via factoring, linear systems via Gaussian)
- Matrix operations (determinant, inverse, eigenvalues, eigenvectors, rank)
- Assumption-based reasoning (sign inference, transitive, De Morgan)
- LaTeX output, file loading, interactive REPL with readline

## What the Rust Kernel CANNOT Do (vs Full Maxima)

- Trig simplification identities (sin²+cos²=1, angle sums, etc.)
- Full Risch integration algorithm (decidability)
- Definite integration via residues/contour methods
- Full limit algorithm (Gruntz MRV for nested exponentials)
- Berlekamp/Hensel factoring (mod p → lift to Z)
- Multivariate polynomial GCD
- Multivariate Gröbner bases
- Special functions (gamma, bessel, elliptic, hypergeometric)
- Numerical methods (SLATEC, QUADPACK, ODE solvers)
- 2D ASCII display (fraction bars, exponent layout)
- Plot/graph output
- Share packages (tensor, draw, cobyla, etc.)
- Interactive asksign prompting

## Metrics

| Metric | Value |
|--------|-------|
| Unit + integration tests | **528** |
| PRs merged | **70** |
| Crates | 5 |
| Modules | 15 |
| Rust LOC | ~12,900 |
| Built-in functions | ~145 |
| Integration patterns | ~50 |
| Compiler warnings | 0 |
| REPL | Readline with arrow keys, history, syntax highlighting |
