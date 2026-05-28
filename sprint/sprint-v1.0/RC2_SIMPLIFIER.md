# RC2 — Simplifier + Polynomial Arithmetic

**Goal:** Implement the simplification engine and basic polynomial
arithmetic. Pass `rtest1.mac`–`rtest4.mac`.

**Exit criteria:** Simplification rules produce correct canonical forms;
polynomial operations (expand, factor basics) work; 4 rtest files pass.

---

## Sprint 2.1 — Simplifier Framework

**Duration:** 3 weeks

### Tasks

- [ ] Implement simplification dispatch table:
  - Each operator registers a simplification function
  - `simplify(expr) → expr` recursively simplifies bottom-up
  - Track `simplified` flag to avoid re-simplification
- [ ] Core simplification rules for `+`:
  - Identity: `x + 0 → x`
  - Combine numeric terms: `3 + 4 → 7`
  - Collect like terms: `2*x + 3*x → 5*x`
  - Flatten: `(a + b) + c → a + b + c`
  - Sort: canonical ordering of terms (numbers first, then alphabetical)
- [ ] Core simplification rules for `*`:
  - Identity: `x * 1 → x`
  - Zero: `x * 0 → 0`
  - Combine numeric factors: `3 * 4 → 12`
  - Collect like bases: `x * x → x^2`, `x^2 * x^3 → x^5`
  - Flatten and sort
- [ ] Core simplification rules for `^`:
  - `x^0 → 1`, `x^1 → x`, `0^n → 0` (n>0), `1^n → 1`
  - Numeric: `2^3 → 8`
  - Power of power: `(x^a)^b → x^(a*b)` (when valid)
- [ ] Core simplification rules for `-` and `/`:
  - `a - b → a + (-1)*b`
  - `a / b → a * b^(-1)`
- [ ] Simplification of equations (`=`), inequalities (`<`, `>`, etc.)

### Tests

```
#[test] fn simp_add_zero()    { simplify("x+0")    == "x" }
#[test] fn simp_add_nums()    { simplify("3+4")     == "7" }
#[test] fn simp_collect()     { simplify("2*x+3*x") == "5*x" }
#[test] fn simp_mul_zero()    { simplify("x*0")     == "0" }
#[test] fn simp_mul_one()     { simplify("x*1")     == "x" }
#[test] fn simp_power_zero()  { simplify("x^0")     == "1" }
#[test] fn simp_power_num()   { simplify("2^10")    == "1024" }
#[test] fn simp_flatten()     { simplify("(a+b)+c") == "c+b+a" }
#[test] fn simp_like_bases()  { simplify("x^2*x^3") == "x^5" }
#[test] fn simp_neg()         { simplify("x-x")     == "0" }
```

---

## Sprint 2.2 — Expand and Trigonometric Basics

**Duration:** 2 weeks

### Tasks

- [ ] `expand(expr)` — distribute multiplication over addition:
  - `(a+b)*(c+d) → a*c + a*d + b*c + b*d`
  - `(a+b)^n` for integer n — binomial expansion
- [ ] `ratexpand(expr)` — expand and collect rational expressions
- [ ] Basic trig simplification (sin, cos as known functions):
  - `sin(0) → 0`, `cos(0) → 1`
  - `sin(%pi) → 0`, `cos(%pi) → -1`
  - Special values at multiples of `%pi/6`, `%pi/4`, `%pi/3`
- [ ] Constants: `%pi`, `%e`, `%i` as special symbols
  - `%e^0 → 1`
  - `log(%e) → 1`
  - `%i^2 → -1`
- [ ] `sqrt(x)` as `x^(1/2)`, simplified for perfect squares
- [ ] `abs(x)` simplification for known-sign arguments

### Tests

```
#[test] fn expand_product()   { expand("(a+b)*(c+d)") == "a*d+a*c+b*d+b*c" }
#[test] fn expand_power()     { expand("(x+1)^3") == "x^3+3*x^2+3*x+1" }
#[test] fn sin_zero()         { simplify("sin(0)") == "0" }
#[test] fn cos_pi()           { simplify("cos(%pi)") == "-1" }
#[test] fn sin_pi_6()         { simplify("sin(%pi/6)") == "1/2" }
#[test] fn sqrt_perfect()     { simplify("sqrt(16)") == "4" }
#[test] fn imaginary_sq()     { simplify("%i^2") == "-1" }
```

---

## Sprint 2.3 — Rational Number Arithmetic

**Duration:** 2 weeks

### Tasks

- [ ] Rational number type: `p/q` stored in reduced form
  - GCD-based reduction on construction
  - Arithmetic: `+`, `-`, `*`, `/`, `^` for rationals
  - Comparison operators
  - Display as `p/q` (not as float)
- [ ] `ratsimp(expr)` — simplify over common denominator
- [ ] `num(expr)`, `denom(expr)` — extract numerator/denominator
- [ ] `float(expr)` — convert exact to floating-point
- [ ] `rationalize(expr)` — convert float to rational approximation
- [ ] Mixed arithmetic: integer + rational, rational + float promotion

### Tests

```
#[test] fn rat_reduce()   { simplify("6/4")       == "3/2" }
#[test] fn rat_add()      { simplify("1/3 + 1/6") == "1/2" }
#[test] fn rat_mul()      { simplify("2/3 * 3/4") == "1/2" }
#[test] fn rat_power()    { simplify("(2/3)^3")   == "8/27" }
#[test] fn rat_neg()      { simplify("1/3 - 1/3") == "0" }
#[test] fn ratsimp_expr() { ratsimp("1/x + 1/y")  == "(y+x)/(x*y)" }
#[test] fn float_conv()   { run("float(1/3);")     == ".3333333333333333" }
```

---

## Sprint 2.4 — rtest1–rtest4 Compatibility

**Duration:** 2 weeks

### Tasks

- [ ] Review rtest2.mac, rtest3.mac, rtest4.mac for required features
- [ ] Implement missing built-in functions as discovered:
  - `subst`, `ratsubst`, `substitute`
  - `diff` (symbolic differentiation — basic cases)
  - `integrate` (basic antiderivatives: polynomials, trig, exp)
  - `limit` (stub or basic cases)
  - `taylor` (stub or basic polynomial expansion)
  - `trigexpand`, `trigsimp`
  - `coeff`, `hipow`, `lopow`
  - `gcd`, `mod`, `remainder`
- [ ] Fix edge cases found by running rtests
- [ ] Track and report pass rates per file

### Tests

```
#[test] fn rtest1() { assert_rtest_passes("tests/rtest1.mac"); }
#[test] fn rtest2() { assert_rtest_passes("tests/rtest2.mac"); }
#[test] fn rtest3() { assert_rtest_passes("tests/rtest3.mac"); }
#[test] fn rtest4() { assert_rtest_passes("tests/rtest4.mac"); }
```

---

## Deliverable

```
$ maxima-kernel
(%i1) expand((x+y)^3);
(%o1)                y^3+3*x*y^2+3*x^2*y+x^3
(%i2) ratsimp(1/(x+1) + 1/(x-1));
(%o2)                    2*x/(x^2-1)
(%i3) diff(x^3, x);
(%o3)                       3*x^2
(%i4) 2/3 + 5/6;
(%o4)                        3/2
```
