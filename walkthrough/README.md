# Walkthrough Examples

Interactive tutorials for the Maxima kernel. Each file is a self-contained
`.mac` script that can be run in batch mode.

## Running

```sh
# Run a single walkthrough
cargo run --bin maxima-repl -- -b walkthrough/01_arithmetic.mac

# Or from a built binary
maxima-repl -b walkthrough/03_calculus.mac
```

## Topics

| # | File | Topic |
|---|------|-------|
| 01 | `01_arithmetic.mac` | Integers, rationals, floats, big numbers, number theory |
| 02 | `02_algebra.mac` | Expand, factor, ratsimp, partfrac, GCD, trig identities |
| 03 | `03_calculus.mac` | Differentiation, indefinite/definite integration, Taylor series |
| 04 | `04_solving.mac` | Polynomial solving, quadratic formula, linear systems |
| 05 | `05_limits.mac` | Limits at points and infinity, indeterminate forms |
| 06 | `06_matrices.mac` | Determinant, inverse, eigenvalues, charpoly |
| 07 | `07_summation.mac` | Closed-form sums (Gosper), binomial coefficients, products |
| 08 | `08_advanced_integration.mac` | Algebraic integrands, Gaussian, Laplace transforms |
| 09 | `09_assumptions.mac` | assume/forget, is, abs simplification, boolean logic |
| 10 | `10_programming.mac` | Functions, blocks, loops, recursion, lambda, lists |
| 11 | `11_file_io.mac` | load, require, save, autoload, search paths |
| 12 | `12_plugin_api.mac` | Rust NativeFn plugin interface (reference) |
| 13 | `13_latex_output.mac` | tex() for LaTeX rendering |
| 14 | `14_matrix_applications.mac` | Matrix power (^^), Fibonacci, element access |
| 15 | `15_game_solver.mac` | 24-game solver (recursive programming showcase) |
| 16 | `16_number_theory.mac` | floor, ceiling, round, mod, gcd, primep |
| 17 | `17_sets.mac` | {}-syntax, union, intersection, powerset |
| 18 | `18_strings.mac` | slength, split, substring, ssearch, parse_string |
| 19 | `19_number_theory.mac` | ifactors, totient, fibonacci, CRT, Jacobi |
| 20 | `20_laplace.mac` | Laplace transforms and inverse (ilt) |
| 21 | `21_ode.mac` | ODE solver (ode2): 1st/2nd order, undetermined coeffs, variation of parameters, ic1/ic2/bc2 |
| 22 | `22_complex.mac` | Complex numbers: %i, realpart, conjugate, rectform |
| 23 | `23_trig_special.mac` | Exact trig values: sin(%pi/6), cos(%pi/4), ... |
| 24 | `24_matrix_arithmetic.mac` | Matrix +/-, scalar*, dot product |
| 25 | `25_partfrac_advanced.mac` | Partial fractions with irreducible quadratics |
| 26 | `26_realroots.mac` | Sturm chains: nroots, realroots |
| 27 | `27_pattern_matching.mac` | matchdeclare, defrule, apply1 |
| 28 | `28_bfloat.mac` | Floating-point evaluation (bfloat) |
| 29 | `29_plotting.mac` | plot2d (SVG), gnuplot_script |
| 30 | `30_ac_matching.mac` | AC pattern matching: commutative sums/products, subset rewrite, rest vars |
| 31 | `31_symbolic_poly.mac` | Symbolic-coefficient resultant & discriminant |
| 32 | `32_residues.mac` | Residues at simple, complex, and higher-order poles |
| 33 | `33_trig_advanced.mac` | trigrat, extended trigreduce, halfangles |
| 34 | `34_rust_plugins.mac` | Dynamic Rust plugins: load_plugin, authoring kit (reference) |
| 35 | `35_orthopoly.mac` | Orthogonal polynomials plugin: Legendre, Chebyshev, Hermite, Laguerre, Jacobi, Gegenbauer |
| 36 | `36_specfun.mac` | Special functions plugin: gamma, beta, erf/erfc, bessel_j/i/y/k |
| 37 | `37_eight_queens.mac` | 8-queens (perms + diagonal filter; recursive backtracking) |
| 38 | `38_queens_visualization.mac` | 8-queens visualization: ASCII boards + scoreboard + reflection (challenge demo) |
| 39 | `39_polynomial_systems.mac` | Polynomial systems: Gröbner basis, polysys_solve, eliminate, ideal arithmetic (V8.0) |
| 40 | `40_sudoku_visualize.mac` | Sudoku visualization: 9x9 board rendering, validation, and candidate digits |
| 41 | `41_sudoku_solver.mac` | Sudoku solver: recursive backtracking, solution counting, and a 4x4 demo |
| 42 | `42_special_integrals.mac` | Named nonelementary integrals (erf, li, Ei, Si, Ci) and the Lazard–Rioboo–Trager logarithmic part |

## Suggested Order

Start with **01-04** for core CAS features, then **05-07** for analysis,
**08** for advanced integration, **09-11** for system features,
**12-13** for output and extensibility, **14-16** for applications,
**17-19** for data structures, **20-21** for transforms and ODEs,
**22-26** for advanced math, and **27-29** for the meta-system.
