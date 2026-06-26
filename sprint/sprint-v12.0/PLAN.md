# Maxima Rust Kernel v12.0 — Deepening the Research Engines

## Theme

Push the v11.0 research engines past their order-1 / quadratic boundaries
(`research/survey/ALGORITHM_SURVEY.md` §1.3, §3.2–3.4):

1. **Order-≥2 creative telescoping** — find (and where possible solve) the
   linear P-recurrences of D-finite definite sums and parametric integrals.
   Many classical sequences (Franel `Σ C(n,k)³`, central Delannoy, Apéry) have
   **no** elementary closed form but satisfy an order-2 recurrence — the
   recurrence *is* the answer.
2. **Algebraic integration beyond quadratics** — Trager/Hermite on cubic+ and
   genus-1 (elliptic) curves; decide elementarity.

**Discipline unchanged:** sampled/guessed results are exactly verified before
return; a correct recurrence or a faithful "nonelementary" beats a wrong closed
form.

## Sprints

| Sprint | Content | Status |
|--------|---------|--------|
| **T1** | `find_recurrence(expr,n)` — minimal linear P-recurrence of a D-finite sequence via exact sampling + null-space, verified. (Zeilberger-package spirit.) | ✅ |
| **T2** | `solve_rec(expr,n)` — closed form of C-finite (constant-coeff) sequences via characteristic roots. | ✅ (C-finite, rational roots) |
| **T3** | Trager/Hermite on cubic+ curves: ∫P(x)/√C (deg C≥3) — elementary R·√C iff reducible, else nonelementary. | ✅ (∫P/√C case) |
| **T4** | `gosper_certificate(t,k)` — Gosper/WZ certificate of an indefinite sum, symbolically verified. | ✅ (indefinite/order-0) |

## Targets

```
find_recurrence(sum(binomial(n,k)^3,k,0,n), n)        → Franel order-2 recurrence
find_recurrence(sum(binomial(n,k)*binomial(n+k,k),k,0,n), n)  → Delannoy
/* T3 */
integrate((x^2+1)/sqrt(x^3+x), x)                     → elementary
integrate(1/sqrt(x^3+1), x)                           → NONELEMENTARY (noun)
```

- **T4** — ✅ (indefinite case) `gosper.rs` `gosper_certificate(t,k)`: the
  rational R(k) with antidifference T(k)=R(k)·t(k), proving Σt(k)=T(b+1)−T(a).
  The certifying identity R(k+1)·r(k)−R(k)=1 is checked SYMBOLICALLY (rigorous),
  with numeric telescoping fall-back for factorial terms.
  `gosper_certificate(k*k!,k)=1/k`, `(k,k)→(k-1)/2`, `1/(k(k+1))→-(k+1)`;
  non-summable (1/k) → noun. (Full order-≥2 Zeilberger certificate needs
  symbolic-parameter Gosper / bivariate CRE over Q[n] — a larger effort.)
- **T2** — ✅ (C-finite case) `recurrence.rs` `solve_rec(expr,n)`: find the
  recurrence; if constant-coefficient with distinct rational characteristic
  roots, return the closed form Σ A_i r_i^n (A_i from a Vandermonde fit to the
  initial values, verified on held-out samples). `solve_rec(3*2^n-5,n)=-5+3*2^n`,
  `solve_rec(5^n-2*4^n,n)`. Variable-coefficient (Franel, n!) and
  irrational/repeated roots → noun. (Full Petkovšek for variable-coefficient
  hypergeometric solutions, and repeated/irrational roots, remain.)
- **P2** — ✅ Recursive multivariate GCD (primitive PRS over Q) in
  `crates/poly/src/mpoly_recgcd.rs`, replacing the incomplete Kronecker GCD:
  `gcd(x^2-y^2,(x+y)^2)=x+y`, `gcd(x+y,x-y)=1` (coprime detected). Wired into
  `gcd` and into multivariate `ratsimp` cancellation (v10 M3):
  `ratsimp((x^2-y^2)/(x-y))=x+y`, `ratsimp((x^3-y^3)/(x-y))=x^2+x*y+y^2`.
- **T3** — ✅ (∫P/√C case) `integrate.rs` `try_sqrt_curve_integrate`: for ∫P(x)/√(C)
  with deg C ≥ 3, solve the Hermite ansatz R'·C + ½·R·C' = P. Exact solution ⇒
  elementary `R·√C`; else the residual is an elliptic/abelian integral ⇒
  nonelementary noun. `∫x^5/√(x^3+1)` and `∫4x^3/√(x^4+1)` now elementary;
  `∫1/√(x^3+1)`, `∫x/√(x^3+1)`, `∫x^2/√(x^3+x)` correctly noun. Differentiation-
  verified. (Full Trager — log part over algebraic extensions, P/√C with poles —
  remains.)
- **P1** — ✅ binomial → BigInt (part 1). ✅ simplifier `Coef` → BigRational
  (part 2): `simplify_plus` keeps a fast i64 accumulator that promotes to an
  exact BigRational on overflow / first rational / bigint, so integer and
  rational sums never overflow (`sum(1/(k^2+1),k,1,12)` exact, `i64::MAX+i64::MAX`
  exact). Shared `bigint_to_expr`/`bigrat_to_expr` in helpers.rs. Code-reviewed
  (3 finder angles): restored the i64 fast path (~2× on int-heavy loops),
  consolidated duplicated converters. Known limit: a rational whose num AND den
  both exceed i64 has no atomic kernel representation (renders as num*den^-1,
  correct in value) — a kernel BigRational Expr type is the deeper fix.

## Carried-forward backlog

Recursive multivariate GCD + v10 M3 · Meijer-G · Karr/Schneider ΠΣ · Reduce/CAD
· 3rd-gen trait architecture.

## Progress notes

- **T1** — ✅ `recurrence.rs` + `find_recurrence(expr,n)` builtin. Exact
  (`BigRational`) sampling of T(n), homogeneous system over candidate
  recurrences of increasing order/degree, *unique* null-space vector, verified
  on held-out samples → coefficient list `[c_0(n),…,c_J(n)]` (Σ_j c_j(n)·T(n+j)=0).
  Order-1 and order-2 D-finite cases:
  `find_recurrence(sum(binomial(n,k)^3,k,0,n),n)` → Franel order-2 recurrence;
  central Delannoy likewise; `2^n`,`n!`,`ΣC(n,k)^2` order-1. Non-P-finite → noun.
  Sampling bounded (n≤20) and `catch_unwind`-guarded so sequences that overflow
  the kernel's i64 arithmetic degrade to a noun rather than crash. (A proper fix
  — BigInt summation/binomial in the kernel — would lift the bound; noted.)
