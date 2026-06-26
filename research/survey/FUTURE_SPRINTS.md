# Future Sprints — Gap Analysis vs Maxima / Mathematica (v13 planning)

**Method.** Five parallel domain probes ran the current binary
(`target/release/maxima-repl`, v12.1.0) on representative inputs and read the
source, classifying each capability as *works / noun / **wrong***. Findings are
deduped and tiered below. Every proposed sprint keeps the project discipline:
**compute → verify → return; correct-or-noun, never wrong.**

Legend — Effort: S(≤1 day) · M(2–4 d) · L(1–2 wk) · XL(multi-wk). Value: ★–★★★★★.

---

## TL;DR recommendation

The probes found **silent correctness bugs** and **day-one gaps** sitting next to
the research-grade engines we've been building. Recommended order:

1. **Tier 0 (Correctness & credibility)** first — small, high-trust, several are
   *actively wrong output*, not just missing features. ~1 sprint total.
2. Then **Tier 1 foundational engines** — each unblocks 2–3 downstream features
   (radical solver+RootOf, exact real-root isolation, arbitrary-precision
   bigfloat, the `simplified`-flag perf short-circuit, inverse Laplace).
3. **Tier 2/3** (research depth + breadth) as directed.
4. **Tier 4** (hash-consing DAG, F4) only when a large-expression workload
   demands it.

---

## Tier 0 — Correctness & credibility (do first)

These are *wrong answers* or trivially-exposed holes. High trust impact, low effort.

| # | Sprint | What's broken (reproduced) | Eff | Val |
|---|--------|----------------------------|-----|-----|
| 0a | **Parametric/symbolic `linsolve` & `solve`** | `linsolve([x+y=a,x-y=b],[x,y])` → `[x=0,y=0]` **(wrong)**; `solve(a*x=b,x)` → noun. Symbolic Gaussian elimination / Cramer over Expr; report singular vs solve. | S | ★★★★ |
| 0b | **Infinite-sum limit wiring** | `sum(1/(k(k+1)),k,1,inf)` → `1-1/(1+inf)` **(wrong)**; `sum(q^k,k,0,inf)` → `(-1+q^(1+inf))/(-1+q)`. Take `limit(S(m),m,inf)` via the (strong) Gruntz engine instead of substituting `inf`; geometric/telescoping + named-constant table (ζ(2), e). | M | ★★★★ |
| 0c | **Definite-integral `inf` leak** | `integrate(1/(1+x^4),x,minf,inf)` emits literal `inf` in output. Gate the limit-substitution path; route divergent/contour cases to noun (until Tier-2 residue engine). | S–M | ★★★ |
| 0d | **`(-1)^(2*n)` prints as `-1^(2*n)`** | Non-round-trippable, wrong value. `needs_parens_in_power` (expr.rs:135) ignores negative numeric bases. | S | ★★★ |
| 0e | **Expand-before-integrate + `∫x^n`** | `integrate((1-x)^4,x,0,1)` and `integrate(x^n,x)` → noun. Expand integrand in the definite path; add `x^n→x^(n+1)/(n+1)` (n≠−1) with assumptions. | S | ★★★ |
| 0f | **`simplify` honors the `simplified` flag** | `simplify()` (simp.rs:106) re-canonicalizes whole subtree every call → iterated squaring **times out (>60s)**. Early-return on `simplified:true`. (Cheap half of the DAG sprint; lands alone.) | S–M | ★★★★ |
| 0g | **Numeric `fib`/`lucas`; exact `rank`/Sturm** | `fib(10)`→noun (breaks `find_recurrence`/`solve_rec` on Fibonacci); `rank()` uses f64 (wrong on symbolic/large); Sturm not square-free (miscounts repeated roots). | S | ★★ |
| 0h | **Plugin discoverability & `,numer` parse** | `load("specfun")` fails (needs explicit `.so` path); parser panics on `expr,numer` / `,modulus=7` ev-modifiers. | S | ★★ |

---

## Tier 1 — Foundational engines (each unblocks downstream sprints)

| # | Sprint | Gap & approach | Eff | Val | Unblocks |
|---|--------|----------------|-----|-----|----------|
| 1a | **Cubic/quartic radical solve + `RootOf`** | `solve(x^3-2,x)`→noun. Biquadratic + Cardano + Ferrari → nested radicals; `RootOf(poly,idx)` + isolating interval as fallback. | M | ★★★★★ | `polysys_solve` (cascade fix), algebraic numbers, eigenvalues |
| 1b | **Exact real-root isolation (rational Sturm/Descartes)** | `realroots(x^2-2)` returns imprecise floats; `sturm`/whole-line `nroots` unexposed. Square-free + exact Sturm at rational endpoints → certified isolating intervals. | M | ★★★★ | inequalities, CAD, RootOf indexing, sign-determination |
| 1c | **Real arbitrary-precision bigfloat** | `bfloat(%pi)` at fpprec:50 → 16 digits (**fake**, pure f64). Add `astro-float`/`dashu` backend; `Expr::BigFloat`; elementary fns + constants to N digits. | L | ★★★★★ | all numeric sprints at true precision |
| 1d | **Inverse Laplace via residues/PFD** | `ilt(1/(s^2+1),s,t)`→noun (table-only). Partial fractions over factor_poly + reuse `residue()` primitive; conjugate poles → exp·cos/sin. | M | ★★★★ | `desolve` (Laplace method), controls/ODE users |
| 1e | **Hash-consing / structural-sharing DAG** | `Expr` is `Vec<Expr>` with deep `Clone`, no `Rc`/hash/sharing. Migrate children to `Rc<Expr>` + hash-cons table. (Tier-4-scale; 0f is the cheap precursor.) | XL | ★★★★ | all large-expression workloads |

---

## Tier 2 — Research-grade depth

| # | Sprint | Gap & approach | Eff | Val |
|---|--------|----------------|-----|-----|
| 2a | **Order-≥2 Zeilberger (proven certificate)** | `sum(binomial(n,k)^3,…)`→noun (Franel/Apéry). Parametrized Gosper: solve for `z_j` + certificate over Q(n). Completes V12-T4. | L | ★★★★★ |
| 2b | **Harmonic / nested sums (Karr–Schneider ΠΣ)** | `sum(1/k,k,1,n)`→noun. Difference-field over H_n, S-sums; Abramov–Petkovšek summation in finite terms. | L | ★★★ |
| 2c | **Algebraic-number arithmetic + factor over extensions** | `alg_field.rs` is **dead code**. Resultant-based α±β, β·α; Trager factoring over Q(α); equality via minpoly+interval. | L | ★★★ |
| 2d | **Full algebraic Risch / Trager log-part** | `∫1/√(x^3+1)` elliptic (correct noun) but no log-part over algebraic extensions, no `P/√C` with poles. Bronstein Ch.2; Trager. | XL | ★★★ |
| 2e | **Contour/residue definite integrals** | `∫_{-inf}^{inf} cos(x)/(1+x^2)`→noun; real-line rationals emit `inf`. UHP residue sum + Jordan's lemma; `∫_0^{2π}R(cos,sin)` via unit-circle. | L | ★★★★ |

---

## Tier 3 — Breadth (high user-facing surface)

| # | Sprint | Gap | Eff | Val |
|---|--------|-----|-----|-----|
| 3a | **Matrix decompositions** (LU, QR, Cholesky, rref, nullspace, exact rank) | all noun; rank is f64. Bareiss fraction-free + Doolittle/Householder. | M | ★★★★ |
| 3b | **General eigen** (irrational/complex/numeric) | only rational eigenvalues; eigenvectors lossy f64. Quadratic-formula roots + exact-rref vectors; Francis QR for float matrices. | M | ★★★★ |
| 3c | **Special-function numeric eval** (ζ, polygamma, polylog, Lambert W, elliptic, Jacobi) | all noun. Borwein/AGM/Halley kernels; register exact identities (ζ(2)=π²/6). | M | ★★★★ |
| 3d | **Numeric solvers** (`find_root`, quadrature `quad_qags`/`romberg`, ODE `rk`) | all noun. Brent, Gauss–Kronrod, RK4/RKF45. | M | ★★★★ |
| 3e | **Fourier / Fourier-sin/cos transforms** | absent. Table engine like laplace.rs; rational case via 2e. | M | ★★★ |
| 3f | **Variable-coeff 2nd-order ODE** (Euler–Cauchy, reduction of order, Frobenius series) | only constant-coeff. Euler is a one-shot win; Frobenius = general series fallback. | M–L | ★★★ |
| 3g | **`desolve` + linear ODE systems** | absent. Laplace-transform method (needs 1d). | M | ★★ |
| 3h | **`radcan`/`rootscontract`/`sqrtdenest`** | `radcan` is a pass-through stub. Root-contraction + denesting (Borodin–Fagin–Tarjan). | L | ★★★ |
| 3i | **Assumption-aware simplification** | `integerp(declared n)`→false; sign doesn't propagate through products/powers; integer·π trig not wired. | M | ★★★ |
| 3j | **User rule engine** (`tellsimpafter`, `let`/`letsimp`, `gensym`, `defmatch`) | mostly absent; `gensym` needed for hygiene. | M | ★★ |
| 3k | **Generating functions / holonomic→GF** | `sum(k*x^k,k,1,inf)`→noun; no `powerseries`. C-finite→rational GF reuses `solve_rec` characteristic poly. | M | ★★ |
| 3l | **Inequality solving** (`solve_rat_ineq`, `fourier_elim`) | absent. Sign-table over isolated roots (needs 1b); Fourier–Motzkin. | M | ★★ |

---

## Tier 4 — Heavy infrastructure (defer until demanded)

| # | Sprint | Gap | Eff | Val |
|---|--------|-----|-----|-----|
| 4a | **F4 Gröbner** (+ Buchberger chain criterion) | Buchberger-only, i64 coeffs. Chain criterion is cheap; F4 needs BigInt + Macaulay matrix. | XL | ★★ |
| 4b | **CAD / quantifier elimination** | absent. Collins CAD over the real-root-isolation + algebraic-number stack. | XL | ★★ |

---

## Decision support — suggested bundles

- **"Trust & polish" release (v12.2):** Tier 0 (0a–0h). ~1 sprint, all small,
  fixes every *wrong-answer* bug + the perf timeout. Strongly recommended regardless.
- **"Solve & numbers" arc (v13):** 1a → 1b → 1c, then 3a/3b/3c/3d. Turns the
  kernel into a credible numeric+solving CAS; 1a alone unblocks several.
- **"Summation completion" arc:** 2a → 2b → 3k. Finishes the creative-telescoping
  story V12 started.
- **"Analysis" arc:** 1d → 2e → 3e → 3f/3g. Transforms + definite integrals + ODEs.

## Resources each sprint will need (to be filled per chosen sprint)

- **Math/reference:** named in each `approach` (A=B, Bronstein, Cohen, Golub & Van
  Loan, Basu–Pollack–Roy, DLMF, QUADPACK, Faugère).
- **Crates:** `astro-float`/`dashu` (1c), criterion (benchmarks, 1e/0f).
- **Tests:** every sprint specifies a verification (differentiate-back, numeric
  cross-check vs mpmath/Maxima, round-trip, residual bounds). A shared
  `tests/fixtures/` table vs Maxima reference outputs is worth standing up.
- **Simulation/validation designs:** numeric cross-check harness (quadrature vs
  closed form; isolating-interval contains-exactly-one-root checks; minpoly
  irreducibility/degree checks for algebraic numbers).
