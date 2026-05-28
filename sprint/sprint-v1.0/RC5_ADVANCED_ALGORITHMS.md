# RC5 Advanced Algorithms: Detailed Implementation Plan

## 1. Hermite Reduction (Rational ∫ with Repeated Factors)

### What It Solves
`∫ p(x)/q(x) dx` where `q` has repeated factors, e.g. `∫ 1/(x+1)^3 dx`,
`∫ (x^2+1)/((x-1)^2*(x+2)) dx`.

### Sub-tasks

**1a. Investigate** (reading existing Maxima code)
- [x] Read `risch.lisp` — uses CRE form, `ratqu` for rational quotients
- [x] Understand: Hermite reduction splits ∫p/q into
  rational part R(x) + ∫ simpler_form dx (where simpler has square-free denom)

**1b. Design**
- Input: `Poly` numerator p, `Poly` denominator q (from poly crate)
- Step 1: Square-free decomposition of q: `q = q1 * q2^2 * q3^3 * ...`
- Step 2: For each repeated factor qi^k (k≥2):
  - Use extended Euclidean algorithm on `qi` and `qi'` (derivative)
  - Find `s, t` such that `s*qi + t*qi' = p_remaining`
  - Extract: `-t/((k-1)*qi^(k-1))` as rational part
  - Continue with reduced integral
- Step 3: Remaining integral has square-free denominator → Rothstein-Trager
- Output: rational_part + ∫ reduced_form dx

**1c. Develop**
- [ ] `hermite_reduce(num: &Poly, den: &Poly) -> (Expr, Poly, Poly)`
  Returns (rational_part, reduced_num, reduced_den)
- [ ] Extended Euclidean algorithm for polynomials:
  `extended_gcd(a, b) -> (gcd, s, t)` where `s*a + t*b = gcd`
- [ ] Wire into `table_integrate` for rational expressions

**1d. Test**
```
∫ 1/(x+1)^2 dx → -1/(x+1)
∫ 1/(x+1)^3 dx → -1/(2*(x+1)^2)
∫ (2*x+1)/(x^2+1)^2 dx → -1/(x^2+1) + atan(x)  [mixed]
∫ x/(x-1)^2 dx → 1/(x-1) + log(x-1)
```

---

## 2. Rothstein-Trager (Logarithmic Part of Rational Integration)

### What It Solves
`∫ p(x)/q(x) dx` where `q` is square-free. The result is a sum of
logarithms: `Σ ci * log(ui(x))`.

### Sub-tasks

**2a. Investigate**
- [x] Maxima uses resultants to find log coefficients
- [x] `res_x(p - t*q', q)` gives a polynomial in `t`
- [x] Roots of this resultant are the log coefficients ci
- [x] For each root: `ui = gcd(p - ci*q', q)`

**2b. Design**
- Input: square-free `Poly` num `p`, `Poly` den `q`
- Step 1: Compute `q' = derivative(q)`
- Step 2: Form `p - t*q'` as a bivariate polynomial (treat t as parameter)
- Step 3: Compute resultant `R(t) = res_x(p - t*q', q)`
  - For univariate: resultant = determinant of Sylvester matrix
  - Simpler: evaluate R(t) = Π q(ri) where ri are roots (Bezout's theorem)
- Step 4: Factor R(t) to find rational roots ci
- Step 5: For each ci: `ui = gcd(p - ci*q', q)`, result += `ci * log(ui)`
- Simplification: For our rational root finder, this works when ci ∈ Q

**2c. Develop**
- [ ] `resultant(p: &Poly, q: &Poly) -> Poly`
  Sylvester matrix determinant (reuse matrix_det or Bareiss)
- [ ] `rothstein_trager(num: &Poly, den: &Poly) -> Vec<(Coeff, Poly)>`
  Returns list of (coefficient, argument) for log terms
- [ ] Wire into `hermite_reduce` output for the remaining integral

**2d. Test**
```
∫ 1/(x^2-1) dx → (1/2)*log(x-1) - (1/2)*log(x+1)  [= our partfrac]
∫ 1/(x^3-1) dx → log(x-1)/3 + ...  [needs complex roots for full answer]
∫ (2*x)/(x^2+1) dx → log(x^2+1)  [simple case]
```

---

## 3. Gruntz Algorithm (Limits via MRV)

### What It Solves
Limits involving exponential/logarithmic growth where direct substitution
and L'Hôpital fail: `lim exp(x)/x^n`, `lim x*log(x)` as x→0+,
`lim (1+1/x)^x` as x→∞.

### Sub-tasks

**3a. Investigate**
- [x] Maxima's `limit.lisp` tries multiple methods: L'Hôpital, Taylor, sign analysis
- [x] `tlimit.lisp` forces Taylor expansion with recursion depth guard
- [x] Gruntz algorithm: find MRV (Most Rapidly Varying) subexpressions
- [x] Key insight: compare growth rates via `lim log|f|/log|g|`

**3b. Design**

The Gruntz algorithm for `lim f(x)` as `x → ∞`:

```
Algorithm MRV_LIMIT(f, x):
1. Compute MRV(f, x) — the set of subexpressions with maximal growth rate
2. If MRV = {x}: f is a polynomial/rational → use existing limit
3. Choose ω ∈ MRV (the most rapidly varying subexpression)
4. Let ω = exp(g(x)) for some g
5. Rewrite f in terms of ω: f = Σ ci * ω^ei
6. Find leading exponent e0 (the most significant power of ω)
7. If e0 > 0: limit is ±∞ (sign of leading coefficient)
   If e0 < 0: limit is 0
   If e0 = 0: limit = MRV_LIMIT(leading_coefficient, x) [recurse]
```

MRV set computation:
```
MRV(c, x) = {} if c is constant
MRV(x, x) = {x}
MRV(f+g, x) = max_mrv(MRV(f,x), MRV(g,x))
MRV(f*g, x) = max_mrv(MRV(f,x), MRV(g,x))
MRV(exp(f), x) = {exp(f)} if f→∞, else MRV(f, x)
MRV(log(f), x) = MRV(f, x)  [log grows slower than any power]
```

Growth comparison: `f ≻ g` if `lim log|f|/log|g| → ∞`

**3c. Develop**
- [ ] `mrv_set(expr, var) -> Vec<Expr>` — compute MRV subexpressions
- [ ] `compare_growth(f, g, var) -> Ordering` — compare growth rates
- [ ] `rewrite_in_mrv(expr, omega, var) -> (Vec<(Expr, Expr)>)` — express as series in ω
- [ ] `gruntz_limit(expr, var) -> Expr` — main algorithm
- [ ] Handle: exp(x), x^n, log(x), exp(exp(x)), nested
- [ ] Wire into `limit` evaluator when polynomial/rational limit fails

**3d. Test**
```
limit(exp(x)/x^100, x, inf) → inf
limit(x*log(x), x, 0, plus) → 0  [needs variable substitution x=1/t]
limit((1+1/x)^x, x, inf) → %e
limit(exp(-x)*exp(x+exp(-x)), x, inf) → 1  [Gruntz's example]
limit(log(log(x))/log(x), x, inf) → 0
```

---

## Implementation Priority

| Task | Difficulty | Impact | Depends On |
|------|-----------|--------|------------|
| **Hermite reduction** | Medium | High — enables ∫ of all rationals | poly GCD (done) |
| **Extended Euclidean** | Low | Required for Hermite | poly divmod (done) |
| **Rothstein-Trager** | Medium | Completes rational ∫ | Hermite, resultant |
| **Resultant** | Medium | Required for R-T | poly arithmetic (done) |
| **MRV set** | Medium | Core of Gruntz | expression analysis |
| **Growth comparison** | Hard | Core of Gruntz | recursive limits |
| **Gruntz main** | Hard | Handles exp/log limits | MRV, rewrite |

### Recommended order:
1. Extended Euclidean for polynomials (30 min)
2. Hermite reduction (2 hours)
3. Resultant via Sylvester (1 hour)
4. Rothstein-Trager (2 hours)
5. MRV set computation (2 hours)
6. Gruntz limit (3 hours)

**Total estimate: ~10 hours of focused work**
