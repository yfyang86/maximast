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

Revised to lead with the **summation** half (the most tractable, concrete
research-grade win, and the foundation Almkvist–Zeilberger mirrors): Gosper →
Zeilberger → AZ (integrals) → Trager.

| Sprint | Content | Size | Survey ref | Status |
|--------|---------|------|------------|--------|
| **R1** | **Gosper's algorithm** — indefinite hypergeometric summation. Hypergeometric shift-ratio (powers/factorials), Gosper–Petkovšek normal form, key-equation solve, telescoping-verified. Wired into `nusum` and definite `sum`. | Large | §1.5/§3.2 | ✅ |
| **R2** | **Definite hypergeometric summation** — order-1 recurrence detection + closed forms (integer & half-integer shifts), plus a Pochhammer/Gamma/factorial-ratio simplification layer. | Large | §3.2 | ✅ |
| **R3** | **Almkvist–Zeilberger** — the integral analog: hyperexponential integrand → linear ODE for the parameter integral → solve (reuse `ode.rs`). | Large | §1.5 | 📋 |
| **R4** | **Trager** algebraic integration: integral basis + Hermite reduction on `y²=r(x)`, algebraic LRT log part; decide elementarity. | Large | §1.3 | 📋 |

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

## Progress notes

- **R1** — ✅ Gosper's algorithm in `crates/eval/src/gosper.rs`. A structural
  hypergeometric shift-ratio handles polynomial/rational/exponential/factorial
  terms (the generic simplifier won't reduce `2^(k+1)/2^k` or `(k+1)!/k!`);
  Gosper–Petkovšek normal form via dispersion + poly GCD; degree-bounded key
  equation solved over Q (own particular-solution Gaussian elimination — the
  shared solver rejects the free variables Gosper needs); telescoping-verified
  numerically before returning (correct-or-noun). Wired into `nusum` and as a
  fallback in definite `sum`. Also fixed `expr_to_poly` to expand polynomial
  bases under integer powers (e.g. `(k+1)^2`). Examples:
  `nusum(k*k!)=(n+1)!-1`, `nusum(2^k)=2^(n+1)-2`, `sum(1/(k*(k+1)))=1-1/(n+1)`,
  `sum(k^3)=(n*(n+1)/2)^2`.

- **R2** — 🚧 (installment 1) `crates/eval/src/hypersum.rs`: definite
  hypergeometric sums via order-1 recurrence detection. Samples S(n) *exactly*,
  detects the ratio S(n+1)/S(n) = c·(n+a)/(n+b) (integer shifts) by search, and
  telescopes it to a factorial-free closed form S(n)=K·c^n·∏(n+i), numerically
  verified before returning (correct-or-noun). Wired into `sum`. Examples:
  `sum(k*binomial(n,k),k,0,n)=n*2^(n-1)`,
  `sum(k^2*binomial(n,k),k,0,n)=n*(n+1)*2^(n-2)`. Deferred: order ≥2 recurrences,
  certificate-based Zeilberger, and half-integer/Gamma closed forms (e.g.
  `sum(binomial(n,k)^2)=binomial(2n,n)` needs Pochhammer(1/2) — returns noun for
  now, never wrong). Those need a Pochhammer/Gamma + factorial-ratio
  simplification layer (currently absent), planned as R2 installment 2.

- **R2** — ✅ (installment 2) Pochhammer/Gamma/factorial simplification layer
  (`crates/eval/src/gammafn.rs` + builtins): `pochhammer(a,m)` expansion,
  `gamma` at integers and half-integers (Γ(p+1/2)=(2p)!/(4^p p!)·√π),
  `makefact` (binomial/pochhammer/gamma → factorial), `minfactorial` (factorial
  ratios with integer-differing args → finite products, incl. product
  denominators). Extended `hypersum` to half-integer shifts via a
  Pochhammer→factorial duplication formula with the 4^n folded into the base so
  it cancels: `sum(binomial(n,k)^2,k,0,n)=factorial(2n)/factorial(n)^2`
  (= binomial(2n,n); verified). Deferred to V12+: order ≥2 recurrences and full
  certificate-based Zeilberger.
