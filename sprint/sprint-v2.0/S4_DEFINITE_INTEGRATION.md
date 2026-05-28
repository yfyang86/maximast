# S8 — Definite Integration

**Status: ✅ Complete** (originally S4, renumbered to S8 in revised plan)

**Goal:** Implement definite integration with infinite bounds,
residue computation, and known integral formulas.

---

## Implemented

### Antiderivative + Bounds (eval.rs)
- [x] Finite bounds: `F(b) - F(a)` via `subst` and `meval`
- [x] Upper infinite: `∫_a^∞ f dx` via Gruntz limit of antiderivative
- [x] Double infinite: `∫_{-∞}^∞ f dx` via limits in both directions
- [x] Residue method tried before antiderivative for `(-∞,∞)` integrals

### Known Definite Integrals
- [x] Gaussian: `∫_{-∞}^∞ exp(-x²) dx = √π`
- [x] Exponential: `∫_0^∞ exp(-ax) dx = 1/a` (for `a > 0`)

### Residue Computation (eval.rs)
- [x] Single irreducible quadratic: `∫ c/(ax²+d) dx = cπ/√(ad)`
- [x] Repeated quadratic `(ax²+c)²`: `kπ/(2c√(ac))`
- [x] Two distinct quadratics: `pπ/(√(a₁a₂)(√a₁+√a₂))`
- [x] Convergence check: `deg(Q) ≥ deg(P) + 2`
- [x] Irreducibility check: `4ac - b² > 0` (no real roots)

### Limit Handler Support
- [x] `atan(+∞) = π/2`, `atan(-∞) = -π/2`
- [x] `inf^(-n) → 0` in power evaluation

## Tests

```
integrate(x^2, x, 0, 1)                      → 1/3            ✅
integrate(sin(x), x, 0, %pi)                 → 2              ✅
integrate(exp(-x), x, 0, inf)                → 1              ✅
integrate(exp(-x^2), x, minf, inf)           → √π             ✅
integrate(1/(x^2+1), x, minf, inf)           → π              ✅
integrate(1/(x^2+1)^2, x, minf, inf)         → π/2            ✅
integrate(1/((x^2+1)*(x^2+4)), x, minf, inf) → π/6            ✅
```

## Remaining
- [ ] Higher-order pole residues (general formula)
- [ ] Cauchy principal value for real-axis poles
- [ ] Almkvist-Zeilberger creative telescoping
