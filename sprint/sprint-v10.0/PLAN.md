# Maxima Rust Kernel v10.0 — Multivariate Polynomial Engine

## Theme

Build out genuine multivariate polynomial capability. The `poly` crate is
univariate (`Poly` over `Coeff`); a sparse multivariate `MPoly` (over
`BigRational`) already exists for Gröbner bases but has **no division, GCD, or
factoring**. v10.0 adds those — the foundation the original gap analysis
(`research/survey/ALGORITHM_SURVEY.md`) flagged as the biggest structural
limit, blocking multivariate factoring, multivariate GCD, and
symbolic-coefficient algebra.

**Additive, not a rewrite:** everything is built on the existing `MPoly`
alongside the untouched univariate `Poly`. Same discipline as v8/v9 — results
verified before they are returned; correctness over coverage.

## Strategy

`MCoeff = BigRational` (a field), and we already have a solid **univariate**
`poly_gcd` / `factor`. So the multivariate operations reduce to univariate ones
via **Kronecker substitution** (`x_i ↦ t^(D^i)`), with an **exact-division
check** as the correctness gate:
- Kronecker can produce a *spurious* (too-large) GCD/factor. But if the
  inverted multivariate candidate **exactly divides** the inputs, it is
  provably the true GCD (degree argument); if it doesn't, we fall back. So we
  never return a wrong answer — at worst an incomplete one.

## Sprints

| Sprint | Content | Size | Status |
|--------|---------|------|--------|
| **M1** | `MPoly` exact division + verified multivariate GCD (Kronecker); wire `gcd` | Medium | ✅ |
| **M2** | Multivariate factoring (Kronecker: factor the image, recombine via exact-div); wire `factor` | Large | ✅ |
| **M3** | Multivariate `ratsimp`/cancellation using the new GCD; `content`/`primpart` over many vars | Medium | 📋 |

## Targets

```
gcd(x^2-y^2, x-y)            → x-y
gcd(x^2-y^2, x^2+2*x*y+y^2)  → x+y
factor(a^2-b^2)              → (a-b)*(a+b)      (M2)
factor(x^2-y^2)              → (x-y)*(x+y)      (M2)
ratsimp((x^2-y^2)/(x-y))     → x+y              (M3)
```

## Carried-forward backlog

Full Almkvist–Zeilberger/holonomic engine · full Trager · Meijer-G · Karr ΠΣ
summation · general exponential towers · Reduce/CAD.

## Progress notes

- **M1** — ✅ `MPoly::exact_div` (multivariate division, exact iff remainder
  vanishes) and `mpoly_gcd` via Kronecker substitution + exact-division
  verification (correct-or-noun, never wrong). Wired into `gcd`. Known limit:
  Kronecker is not gcd-preserving, so some cases (e.g. `gcd(x²−y²,(x+y)²)`)
  return noun. A proper recursive multivariate GCD is deferred to a later
  sprint (its verification is weaker, so it needs its own careful build).
- **M2** — ✅ `mpoly_factor`: Kronecker substitution → univariate `factor_poly`
  → greedy recombination, each candidate accepted only when it exactly divides.
  Wired into `factor`. Works: `factor(a²−b²)=(a−b)(a+b)`, `x²+2xy+y²=(x+y)²`,
  `x³−y³=(x−y)(x²+xy+y²)`, `ab+a+b+1=(a+1)(b+1)`, numeric content
  (`2x²−2y²=2(x−y)(x+y)`). Known limit: completeness is bounded by the
  univariate `factor_poly` (rational-roots + sqfree only), so cases needing
  higher-degree univariate splitting (e.g. `factor(x⁴−y⁴)` needs `x²+y²`)
  return unfactored — safe, never wrong. A full Zassenhaus univariate
  factorizer would lift this.
