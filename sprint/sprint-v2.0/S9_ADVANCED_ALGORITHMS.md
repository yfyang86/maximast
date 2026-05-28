# S9 — Advanced Algorithms + Remaining Completion

**Status: S9.1 ✅ S9.2 ✅ (in progress)**

**Goal:** Complete all remaining S1-S8 tasks, fix remaining formula failures,
and introduce algebraic integration capabilities.

---

## Sub-sprints

### S9.1 — Fix Remaining Formula Failures (Small, ~2 days)

Raise formula pass rate from 94.9% to ~100%.

**3 remaining failures:**

| # | Formula | Issue | Fix |
|---|---------|-------|-----|
| 179 | `∫ x·eˣ·sin(x)` | Triple product by-parts | By-parts: u=x, dv=eˣsin(x)dx, recurse |
| 180 | `∫ x·eˣ·cos(x)` | Triple product by-parts | By-parts: u=x, dv=eˣcos(x)dx, recurse |
| 183 | `∫ eˣ·tanh(x)` | Mixed exp·hyp | Substitute u=eˣ, tanh=(u²-1)/(u²+1) |

**Implementation:** Add 3-factor product handler in `table_integrate`:
- `x^n * exp(x) * sin(x)`: by-parts with u=x^n, dv=exp·sin (known antiderivative)
- `exp(x) * tanh(x)`: direct formula `exp(x) - 2·atan(exp(x))`

**Tests:** All 3 formulas above

---

### S9.2 — Trig Power-Product Engine (Medium, ~3 days)

Replace scattered trig patterns with a general engine.

- [x] General reduction formulas for sin^n, cos^n, tan^n, cot^n, sec^n, csc^n (n≥4)
- [x] Product-to-sum: sin(ax)·cos(bx), sin(ax)·sin(bx), cos(ax)·cos(bx) (#34-36)
- [x] f^n·g patterns: sec^n·tan, csc^n·cot, cos^n·sin, sin^n·cos, cosh^n·sinh, sinh^n·cosh
- [x] Special products: sec·csc = log|tan|, exp·tanh, 1/cosh·tanh, 1/sinh·coth
- [x] Triple product: x·exp(x)·sin(x), x·exp(x)·cos(x) by-parts
- [x] sin²·cos² = x/8 - sin(4x)/32
- [ ] `integrate_trig_power_product(a, b, pair, var)` for sin^a·cos^b, sec^a·tan^b, etc.
  - Odd exponent: peel one factor, Pythagorean identity, u-substitute
  - Both even: double-angle reduction, recurse
  - Handles arbitrary integer powers (currently only n=2,3,4)
- [ ] Product-to-sum: `sin(ax)·cos(bx)` = `(sin(a+b)x + sin(a-b)x)/2` (formulas #34-36)
- [ ] Reduction formulas: `∫ sin^n = -sin^(n-1)·cos/n + (n-1)/n·∫ sin^(n-2)` (formulas #28-33)

**Tests:** `sin^5`, `sin^2·cos^3`, `sec^4`, `sin(2x)·cos(3x)`

---

### S9.3 — Rational + Risch Completion (Large, ~5 days)

Complete the remaining S4/S5 algorithmic debt.

- [ ] **Full Lazard-Rioboo-Trager** in `poly/hermite.rs`
  - `resultant_x(p - t·q', q)` → factor → extract log coefficients
  - Handles algebraic number roots in the resultant
  - **Test:** `∫ 1/(x³+1) dx`

- [ ] **Repeated quadratic factors** (general case)
  - Recursive reduction: `∫ P/(Q^n) = rational_part + ∫ P'/(Q^(n-1))`
  - Currently only `(ax²+c)²` with `b=0` handled
  - **Test:** `∫ 1/(x²+1)³`, `∫ x/(x²+x+1)²`

- [ ] **Polynomial reduction in tower** in `risch_integrate.rs`
  - Extend to handle degree > 1 polynomials in tower variable t
  - Hermite reduction within the tower field
  - **Test:** `∫ log(x)³/x = log(x)⁴/4`

---

### S9.4 — Gruntz Completion + Zeilberger (Medium, ~4 days)

- [ ] **Full series composition** in `series.rs`
  - `series_compose(outer, inner)` for `exp(series)`, `log(1+series)`, `sqrt(1+series)`
  - Enables proper rewrite step cancellation
  - **Test:** `limit(exp(x+exp(-x))-exp(x), x, inf) = 1` (Gruntz classic)

- [ ] **Sqrt conjugate** in `gruntz.rs`
  - `sqrt(x²+1)-x`: rationalize to `1/(sqrt(x²+1)+x) → 0`
  - **Test:** `limit(sqrt(x²+1)-x, x, inf) = 0`

- [ ] **Zeilberger creative telescoping** in eval.rs
  - Given `F(n,k)` hypergeometric in both n and k
  - Find recurrence `p₀(n)·S(n) + p₁(n)·S(n+1) + ... = 0`
  - Uses Gosper as subroutine (already implemented)
  - **Test:** `sum(binomial(n,k), k, 0, n) = 2^n`

- [ ] **Higher-order pole residues** (general formula)
  - `Res(f, z₀) = (1/(n-1)!) · lim d^(n-1)/dz^(n-1) [(z-z₀)^n · f(z)]`
  - **Test:** `∫ 1/(x²+1)³ [-∞,∞] = 3π/8`

- [ ] **Cauchy principal value**
  - Split at real-axis poles, take symmetric limits
  - **Test:** `PV ∫ 1/x dx [-1,1] = 0`

---

### S9.5 — Risch Algebraic Extensions (Large, ~7 days)

Handle integrands with radicals via algebraic field extensions.

- [ ] **Algebraic number field type** in new `poly/alg_ext.rs`
  - `AlgExt { minpoly: Poly, gen: SymbolId }`
  - Arithmetic: add, mul, inv via extended GCD mod minpoly
  - Implement `Ring` trait

- [ ] **Radical-to-algebraic rewriting**
  - `sqrt(expr)` → algebraic extension `t` where `t² - expr = 0`
  - Extend `risch_tower.rs` with `Extension::Algebraic { minpoly: Poly }`
  - Derivative: `t' = u'/(n·t^(n-1))` for `t^n = u(x)`

- [ ] **Trager's algorithm** for algebraic function integration
  - Hermite reduction in `K(x)[t]/(t^n - u)`
  - Logarithmic part via resultant over algebraic extension
  - **Test:** `∫ 1/sqrt(x²+1) = asinh(x)`, `∫ sqrt(1-x²) = x·sqrt(1-x²)/2 + asin(x)/2`

- [ ] **Almkvist-Zeilberger creative telescoping**
  - Continuous analogue of Zeilberger for parametric integrals
  - **Test:** `∫₀^∞ x^n·exp(-x) dx = n!`

---

### S9.D — Deferred Infrastructure

- [ ] Hash-consed expression DAG (major refactor, defer to v3.0)
- [ ] CRE benchmark suite (nice-to-have)

---

## Priority Order

```
S9.1 (formula fixes)     ← quick wins, do first
  ↓
S9.2 (trig engine)       ← builds on S9.1 normalization
  ↓
S9.3 (rational + Risch)  ← independent, medium effort
  ↓
S9.4 (Gruntz + summ.)    ← independent, medium effort
  ↓
S9.5 (algebraic)         ← depends on S9.3 (LRT) + poly infrastructure
```

## Success Metrics

- Formula suite: 59/59 (100%) after S9.1
- All 15 remaining S1-S8 tasks resolved after S9.4
- `limit(exp(x+exp(-x))-exp(x), x, inf) = 1` after S9.4
- `integrate(sqrt(1-x²), x)` works after S9.5
- `sum(binomial(n,k), k, 0, n) = 2^n` after S9.4

## Estimated Timeline

| Sub-sprint | Days | Cumulative |
|-----------|------|------------|
| S9.1 | 2 | 2 |
| S9.2 | 3 | 5 |
| S9.3 | 5 | 10 |
| S9.4 | 4 | 14 |
| S9.5 | 7 | 21 |
