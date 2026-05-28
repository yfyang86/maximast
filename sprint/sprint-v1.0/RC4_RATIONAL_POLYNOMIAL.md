# RC4 — Rational Functions + Factoring

**Goal:** Implement canonical rational expression (CRE) representation,
polynomial GCD, factoring, and partial fractions. Pass `rtest5–rtest8`.

**Exit criteria:** `factor(x^2-1)` → `(x-1)*(x+1)`;
polynomial GCD works; 8 cumulative rtest files pass.

---

## Sprint 4.1 — Canonical Rational Expression (CRE)

**Duration:** 3 weeks

### Tasks

- [ ] CRE internal representation:
  - Polynomial: sorted list of `(exponent, coefficient)` pairs
  - Variable ordering: user-controllable, default alphabetical
  - Rational: `(numerator_poly, denominator_poly)`
  - Stored normalized: denominator monic or positive leading coefficient
- [ ] Conversion functions:
  - `rat(expr)` — general expression → CRE
  - `ratdisrep(cre)` — CRE → general expression
- [ ] CRE arithmetic:
  - Addition, subtraction (align terms, add coefficients)
  - Multiplication (convolution of coefficient lists)
  - Division (polynomial long division + remainder)
  - Exponentiation (repeated squaring)
- [ ] `ratp(expr)` — test if expression is in CRE form
- [ ] `ratexpand`, `ratsimp` using CRE internally

### Tests

```
#[test] fn rat_convert()      { run("rat(x^2+2*x+1);") contains CRE form }
#[test] fn rat_add()          { run("rat(x+1) + rat(x-1);") == "2*x" }
#[test] fn rat_mul()          { run("rat(x+1) * rat(x-1);") == "x^2-1" }
#[test] fn rat_div()          { run("divide(x^3-1, x-1);") == "[x^2+x+1, 0]" }
#[test] fn ratexpand_works()  { run("ratexpand((x+1)^2);") == "x^2+2*x+1" }
```

---

## Sprint 4.2 — Polynomial GCD and Factoring

**Duration:** 3 weeks

### Tasks

- [ ] Polynomial GCD algorithms:
  - Euclidean algorithm for univariate polynomials
  - Subresultant GCD (avoids coefficient explosion)
  - Multivariate: recursive reduction to univariate
- [ ] `gcd(p, q)` — polynomial GCD
- [ ] `content(poly)` / `primpart(poly)` — content and primitive part
- [ ] Factoring:
  - `factor(expr)` — factor polynomials over integers
  - Square-free factorization (Yun's algorithm)
  - Berlekamp or Cantor–Zassenhaus for factoring mod p
  - Hensel lifting for factors over Z
  - Kronecker's method as fallback for small degrees
- [ ] `sqfr(expr)` — square-free decomposition
- [ ] `resultant(p, q, var)` — resultant of two polynomials
- [ ] `discriminant(poly, var)`

### Tests

```
#[test] fn factor_diff_sq()   { factor("x^2-1")     == "(x-1)*(x+1)" }
#[test] fn factor_cubic()     { factor("x^3-1")     == "(x-1)*(x^2+x+1)" }
#[test] fn factor_quartic()   { factor("x^4-1")     == "(x-1)*(x+1)*(x^2+1)" }
#[test] fn factor_coeff()     { factor("2*x^2+4*x+2") == "2*(x+1)^2" }
#[test] fn factor_bivariate() { factor("x^2-y^2")   == "(x-y)*(x+y)" }
#[test] fn gcd_poly()         { run("gcd(x^2-1, x^2+2*x+1);") == "x+1" }
#[test] fn sqfr_basic()       { run("sqfr(x^3-3*x^2+3*x-1);") == "(x-1)^3" }
#[test] fn resultant_basic()  { run("resultant(x+y, x-y, x);") == "-2*y" }
```

---

## Sprint 4.3 — Partial Fractions and Rational Simplification

**Duration:** 2 weeks

### Tasks

- [ ] `partfrac(expr, var)` — partial fraction decomposition
- [ ] `combine(expr)` — combine fractions over common denominator
- [ ] `ratsimp` improvements:
  - Cancel common factors in num/denom
  - Simplify nested fractions
- [ ] `ratvars(v1, v2, ...)` — set variable ordering for CRE
- [ ] `tellrat(expr)` — declare algebraic relations
- [ ] `algebraic: true` flag and algebraic simplification

### Tests

```
#[test] fn partfrac_basic()  { run("partfrac(1/(x^2-1), x);") == "1/(2*(x-1))-1/(2*(x+1))" }
#[test] fn partfrac_repeat() { run("partfrac(1/(x^2*(x+1)), x);") == ... }
#[test] fn combine_basic()   { run("combine(1/x + 1/y);") == "(y+x)/(x*y)" }
#[test] fn ratsimp_cancel()  { run("ratsimp((x^2-1)/(x-1));") == "x+1" }
```

---

## Sprint 4.4 — rtest5–rtest8 Compatibility

**Duration:** 2 weeks

### Tasks

- [ ] Run `rtest5.mac`–`rtest8.mac`, identify missing features
- [ ] Implement as needed:
  - `remainder`, `quotient`
  - `coeff`, `hipow`, `lopow`
  - `ratcoeff`, `ratsubst`
  - `xthru` — simplify by multiplying through
  - `multthru`
- [ ] Fix edge cases and regressions in rtest1–4
- [ ] Performance: ensure polynomial operations on degree-50+ polynomials
  complete in reasonable time (<1s)

### Tests

```
#[test] fn rtest5() { assert_rtest_passes("tests/rtest5.mac"); }
#[test] fn rtest6() { assert_rtest_passes("tests/rtest6.mac"); }
#[test] fn rtest7() { assert_rtest_passes("tests/rtest7.mac"); }
#[test] fn rtest8() { assert_rtest_passes("tests/rtest8.mac"); }
```

---

## Deliverable

```
$ maxima-kernel
(%i1) factor(x^4 - 1);
(%o1)                (x-1)*(x+1)*(x^2+1)
(%i2) partfrac(1/(x^3-x), x);
(%o2)            1/(2*(x-1))-1/x-1/(2*(x+1))
(%i3) gcd(x^4-1, x^6-1);
(%o3)                     x^2-1
(%i4) ratsimp((x^3-1)/(x-1));
(%o4)                   x^2+x+1
```
