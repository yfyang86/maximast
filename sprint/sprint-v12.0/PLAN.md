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
| **T2** | Solve found recurrences to closed form when possible (order-1 already; order-2 hypergeometric via Petkovšek/d'Alembertian); wire into `sum`/`integrate`. | 📋 |
| **T3** | Trager on cubic/genus-1 curves: integral basis + Hermite reduction; decide elementarity (e.g. `∫(x²+1)/√(x³+x)` elementary vs `∫1/√(x³+1)` nonelementary). | 📋 |
| **T4** | Certificate-based proof: turn a sampled recurrence into a verified telescoping certificate. | 📋 |

## Targets

```
find_recurrence(sum(binomial(n,k)^3,k,0,n), n)        → Franel order-2 recurrence
find_recurrence(sum(binomial(n,k)*binomial(n+k,k),k,0,n), n)  → Delannoy
/* T3 */
integrate((x^2+1)/sqrt(x^3+x), x)                     → elementary
integrate(1/sqrt(x^3+1), x)                           → NONELEMENTARY (noun)
```

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
