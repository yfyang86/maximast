# Maxima Rust Kernel v11.0 — Research-Grade Symbolic Engines

## Theme

Finish the hard symbolic deferrals from v8.0 — the genuine FriCAS-class
frontier where Maxima is weakest and no Rust competitor exists
(`research/survey/ALGORITHM_SURVEY.md` §1.3, §1.5, §3.2–3.4, §5.1):

1. **Holonomic / D-finite closure + Almkvist–Zeilberger** — definite
   integrals and sums of D-finite functions via creative telescoping.
2. **Trager algebraic integration** — elementary integrals of algebraic
   functions (radicals) over curves.

Prerequisite (v10.0): the multivariate polynomial engine — holonomic work
needs multivariate / Ore-algebra arithmetic, and Trager needs polynomials
over algebraic extensions.

**Discipline unchanged:** every closed form numerically verified before it is
returned; noun (or a correct "nonelementary"/recurrence) beats a wrong answer.
These are research-grade (survey effort: AZ ≈4 PM, holonomic 3–5 PM, Trager
6–10 PM), so each sprint may ship a **scoped** subset — flagged honestly, as
v8.0 S5/S6 were.

## Sprints

| Sprint | Content | Size | Survey ref |
|--------|---------|------|------------|
| **R1** | Ore-algebra core: factor the operator/creative-telescoping engine out of `zeilberger.rs` into a shared `ore.rs` (operators, Ore polynomials, reduction) | Medium | §3.2 |
| **R2** | Holonomic closure: represent D-finite functions by (annihilating ODE/recurrence + initial values); closure under +, ×, and `∫` | Large | §3.4 / §5.1 |
| **R3** | Almkvist–Zeilberger: hyperexponential integrand → linear ODE for the parameter integral → solve (reuse `ode.rs`); generalize the special-cased ∫₀^∞ families | Large | §1.5 |
| **R4** | Trager algebraic integration: integral basis + Hermite reduction on `y²=r(x)`, algebraic Lazard–Rioboo–Trager log part; decide elementarity | Large | §1.3 |

### Phasing

| Phase | Sprints | Focus |
|-------|---------|-------|
| **Phase 1 — Infrastructure** | R1 → R2 | The Ore-algebra + holonomic substrate everything else reuses |
| **Phase 2 — Definite integration** | R3 | Almkvist–Zeilberger on the holonomic substrate |
| **Phase 3 — Algebraic frontier** | R4 | Trager, the hardest indefinite case |

## Targets

```
/* Almkvist–Zeilberger (R3) — general, not table-special-cased */
integrate(exp(-a*x^2)*x^(2*n), x, 0, inf);     → parametrised Gaussian moments
integrate(x^s/(1+x), x, 0, inf);               → π/sin(π s) family
/* Holonomic closure (R2) */
ode satisfied by  bessel_j(0,x)*exp(x), etc.
/* Trager (R4) */
integrate(x/sqrt(x^4+1), x);                   → already via subst; now general
integrate((x^2+1)/sqrt(x^3+x), x);             → elementary algebraic case
integrate(1/sqrt(x^3+1), x);                   → correctly NONELEMENTARY (noun)
```

## Carried-forward backlog (beyond v11.0)

Meijer-G tables · Karr/Schneider ΠΣ summation · general Risch exponential
towers · Reduce/CAD quantifier elimination · 3rd-gen trait architecture.

## Open questions (resolve before R3/R4)

| # | Topic | Question |
|---|-------|----------|
| 1 | R2 representation | Annihilator as a single ODE in `d/dx` only, or full Ore (mixed `∂x`/shift) for sums too? Start ODE-only? |
| 2 | R3 scope | Hyperexponential integrands only (achievable), or general D-finite (needs full R2)? |
| 3 | R4 ambition | Radical-quadratic/cubic only this release, or attempt general `y²=r(x)` hyperelliptic? |
| 4 | bignum | Trager resultants over Q can overflow i64 — stay pure `num::BigInt`, or accept an LGPL FLINT fast path for the heavy algebraic work? |
