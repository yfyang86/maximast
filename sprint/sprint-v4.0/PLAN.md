# Maxima Rust Kernel v4.0+ — Algebraic Extensions, Advanced Integration, Package System

## Status: ✅ Complete (2026-05-26)

All sprints implemented through V5.0. 808 tests, zero failures.

---

## Sprint Summary

| Sprint | Content | Status |
|--------|---------|--------|
| **V4.1** | CLI + script runner | ✅ Done |
| **V4.2** | Poly over AlgField (generic ring) | ✅ Done |
| **V4.3** | Algebraic factoring for integration | ✅ Done |
| **V4.3+** | Trager norm shift, cyclotomic Q(ω), i128 overflow protection | ✅ Done |
| **V4.4** | Radical integration patterns | ✅ Done |
| **V4.4+** | Euler substitution, √(x²+c)/x², 1/(x√(a²-x²)) | ✅ Done |
| **V4.5** | Almkvist-Zeilberger (Gamma integral) | ✅ Done |
| **V4.5+** | Laplace transforms, Gaussian-cosine integrals | ✅ Done |
| **V4.6** | File I/O + file_search | ✅ Done |
| **V4.7** | Benchmarks + bug fixes | ✅ Done |
| **V5.0** | Plugin-ready package system | ✅ Done |

---

## V4.1 — CLI + Script Runner

- [x] CLI argument parsing: -e, --batch, file.mac, stdin pipe, --help, --version
- [x] Stdin pipe support: `echo "2+3;" | maxima-kernel`
- [x] Exit code: 0 success, 1 error
- [x] --help and --version flags
- [x] Script return value: last expression's value
- [x] 5 execution modes: REPL, Eval, Batch, Stdin, Help

---

## V4.2 — Polynomial Ring Over AlgField

- [x] Ring trait for AlgNumber
- [x] PolyAlg type with AlgNumber coefficients
- [x] PolyAlg add/sub/mul/divmod/derivative
- [x] PolyAlg gcd via Euclidean algorithm
- [x] PolyAlg from_poly — lift Poly into Q(α)[x]
- [x] make_monic for PolyAlg
- [x] Extended GCD for PolyAlg
- [x] Verified: (x²+√2x+1)(x²-√2x+1) = x⁴+1

---

## V4.3 — Algebraic Factoring for Integration

- [x] Detect x⁴+bx²+c form, compute β=√c, α²=2β-b
- [x] Rational factoring when α² is perfect square (x⁴+x²+1)
- [x] Algebraic factoring over Q(√d) when α² is not perfect square (x⁴+1)
- [x] Integrate each quadratic factor via completing square → log+atan
- [x] `∫ 1/(x⁴+1)` = log((x²+√2x+1)/(x²-√2x+1))/(2√2) + atan terms

**V4.3+ Advanced Algebraic (completed 2026-05-26):**
- [x] General `factor_over_extension(p, field)` via Trager norm shift
- [x] Factor x³-1 over Q(ω) cyclotomic extension
- [x] Cyclotomic polynomial Φₙ(x) generator
- [x] i128 intermediate arithmetic in AlgNumber to prevent overflow
- [x] Degree-3+ extension support for Q(ω) factoring

---

## V4.4 — Radical Integration Patterns

- [x] √x·log(x) → (2/9)x^(3/2)(3log(x)-2) via substitution u=√x
- [x] x²/√(x²+c) → (x/2)√(x²+c) - (c/2)log(x+√(x²+c))
- [x] √(a-x²) → x√(a-x²)/2 + (a/2)asin(x/√a)
- [x] √(x²+a) → x√(x²+a)/2 + (a/2)log(x+√(x²+a))
- [x] 1/√(x²+bx+c) → completing square → asinh/log
- [x] 1/(x·√(x²-c)) → atan(√(x²-c)/√c)/√c

**V4.4+ Advanced Radical (completed 2026-05-26):**
- [x] Euler substitution for ∫ 1/((x+a)√(x²+c)) — all 3 cases (a²>c, a²=c, a²<c)
- [x] √(x²+c)/x² product form via integration table #70
- [x] Bug fix: a²<c case uses log (not atan) — verified by derivative check

**Not implemented (future work):**
- [ ] General Euler substitution for ∫ R(x, √(ax²+bx+c)) dx with arbitrary R
- [ ] Extension::Algebraic in Risch tower
- [ ] Hermite reduction in K(x)[t]/(t²-g)

---

## V4.5 — Almkvist-Zeilberger

- [x] Detect x^n·exp(-a·x) on [0,∞) → factorial(n)/a^(n+1)
- [x] Evaluate factorial for integer n (6, 120, ...)
- [x] Return symbolic factorial(n) for symbolic n
- [x] `∫₀^∞ x^n·exp(-x) dx = n!`
- [x] `∫₀^∞ x^n·exp(-2x) dx = n!/2^(n+1)`

**V4.5+ Advanced Definite Integrals (completed 2026-05-26):**
- [x] Laplace transform table: ∫₀^∞ exp(-sx)cos(bx), exp(-sx)sin(bx)
- [x] Gaussian-cosine: ∫₀^∞ exp(-ax²)cos(bx) for any a>0
- [x] Gaussian x^(2n)·exp(-x²) with i128 overflow protection
- [x] Bug fix: Gaussian overflow for large n (checked_mul, shift bounds)

**Not implemented (future work):**
- [ ] Full parametrized continuous Gosper (Risch DE with parameters)
- [ ] General hyperexponential detection

---

## V4.6 — File I/O

- [x] `file_search(name)` — search ., .mac, share/, tests/ (6 path variants)
- [x] `save("file.mac", var1, var2, ...)` — write variable bindings
- [x] `stringout("file.mac", expr1, expr2, ...)` — write expressions
- [x] `printfile("file.txt")` — display file contents to stdout

**Upgraded in V5.0:**
- [x] Configurable file_search_maxima path list
- [x] `require()` — load-once semantics
- [x] `setup_autoload()` — lazy loading
- [x] `loaded_files()` — list loaded files

**Not implemented (future work):**
- [ ] `restore("file.mac")` — restore saved bindings
- [ ] `writefile("output.txt")` / `closefile()` — output redirection

---

## V4.7 — Benchmarks + Bug Fixes

- [x] Benchmark suite (benchmark.rs): 10 test groups, 70+ checks
  - Arithmetic, Float, Algebra, Solve, Calculus, Limits,
    Summation, Definite Integrals, Matrix, Performance Scaling
- [x] Performance scaling: expand (x+1)^n for n=10,20,50; factor; gcd; sum
- [x] Bug fix: solve(a*x²+b*x+c=0, x) → quadratic formula with √(b²-4ac)
- [x] Bug fix: float(%pi) → 3.14159... (recursive float evaluation)
- [x] Bug fix: solve(x²+3x+2=0, x) → [x=-1, x=-2] (equation form parsing)
- [x] Bug fix: sqrt(1) → 1 in residue formula

**Not implemented (future work):**
- [ ] Criterion benchmark crate for proper microbenchmarks
- [ ] Hash-consed ExprId for simplifier memoization
- [ ] Profiling hot paths

---

---

## V5.0 — Plugin-Ready Package System

- [x] `NativeFn` type alias: `fn(&[Expr], &mut Environment) -> Expr`
- [x] `NativeFuncDef` with min/max arg count validation
- [x] `Environment.register_native(name, func, min_args, max_args)`
- [x] Dispatch order: native → user-defined → lambda → autoload → noun form
- [x] `kill(all)` preserves native functions
- [x] Configurable `search_paths` with nested load support
- [x] `load_pathname` tracking during file loading
- [x] `loaded_files: HashSet` prevents double-loading
- [x] `resolve_file()` searches relative to current load path, then search_paths
- [x] `require("file")` — load-once semantics
- [x] `setup_autoload("file", f1, f2, ...)` — lazy-load on first call
- [x] `loaded_files()` — list all loaded file paths
- [x] `load_pathname()` — current file being loaded
- [x] `file_search_maxima()` — list configured search paths
- [x] 10 integration tests (package_system.rs)
- [x] 16 walkthrough examples (walkthrough/*.mac)
- [x] User manual (user-manual.md)

**Architecture note:** The `NativeFn` + `register_native` API is designed to
support future dynamic plugin loading (`.so`/`.dylib` via `dlopen`). Plugin
crates will compile as `cdylib` and export a registration entry point.

---

## V5.1 — Bug Fixes, REPL, Walkthroughs

- [x] sqrt simplification in evaluator, simplifier, integrator (3 layers)
- [x] Remove wrong Euler substitution formulas (all 3 cases numerically verified as incorrect)
- [x] Fix limit(sin(x)/abs(x), x, 0) — abs-aware directional limit with derivative sign detection
- [x] Fix output format ambiguity — parens for negative exponents, fractions, negation
- [x] Batch mode re-parse bug — evaluate AST directly via `eval_expr_with_env`
- [x] Parser: `for i from 0 thru n` syntax
- [x] floor, ceiling, truncate, round (banker's rounding)
- [x] Matrix indexing M[i,j] and matrix power M^^n
- [x] endcons, second, third, fourth, fifth
- [x] Tab completion with function/keyword/constant lists
- [x] simplify_power arm ordering for (x^(1/2))^n
- [x] checked_mul in sqrt for large integers
- [x] 14 new edge-case tests (sqrt_edge_cases.rs)
- [x] 3 new walkthrough tutorials (matrix applications, 24-game solver, number theory)
- [x] User manual (user-manual.md)
- [x] Code review: 3 parallel agents, all critical/medium findings fixed

---

## Version History

| Version | Tests | Key milestone |
|---------|-------|---------------|
| v1.0 | 326 | Parser, evaluator, simplifier |
| v2.0 | 777 | Hermite, Risch tower, Gruntz MRV, series |
| v3.0 | 777 | AlgField, Gruntz classic, LRT, Zeilberger |
| v4.0 | 794 | CLI, PolyAlg, ∫1/(x⁴+1), Gamma, benchmarks |
| v4.0+ | 798 | Trager norm, Euler sub, Laplace, bug fixes |
| **v5.0** | **822** | **Plugin API, package system, walkthroughs, 11 bug fixes** |
