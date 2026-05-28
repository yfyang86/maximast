# S2/S4 — Complete Rational Integration

**Status: ✅ Complete** (originally S2 + expanded into S4 in revised plan)

**Goal:** Implement full rational function integration via Hermite reduction,
partial fractions, and CRE arithmetic.

---

## Implemented

### Trait Hierarchy (poly/traits.rs)
- [x] `Ring → IntegralDomain → GcdDomain → EuclideanDomain → Field`
- [x] `DifferentialRing` trait
- [x] Implementations for `i64` and `Coeff`

### CRE Type (poly/cre.rs)
- [x] `CRE { num: Poly, den: Poly, var: SymbolId }`
- [x] `CRE::new` — GCD normalization, monic denominator
- [x] `add`, `sub`, `mul`, `div`, `neg`, `derivative`
- [x] `eval_at`, `degree_diff`, `Display`

### CRE ↔ Expr Conversion (poly/convert.rs)
- [x] `expr_to_cre` — detect negative-exponent factors as denominator
- [x] `cre_to_expr` — reconstruct as `num * den^(-1)`

### Poly GCD Fix (poly/gcd.rs)
- [x] `primitive()` normalizes rational-coefficient intermediates to monic integer form
- [x] Fixes `gcd(x³+2x²+x, 3x²+4x+1) = x+1` (was `-2/9*(x+1)`)

### Hermite Reduction (eval.rs)
- [x] Per-factor iterative reduction for linear factors
- [x] Extract residue at root, extended GCD on `q_i` and `q_i'`
- [x] Rational part: `-t_scaled / ((j-1) * q_i^{j-1})`
- [x] Remaining square-free integral via partfrac

### Partial Fractions
- [x] All-linear factors: residue at root → log terms
- [x] Mixed linear + irreducible quadratic factors (`integrate_partfrac_mixed`)
- [x] Quadratic coefficient extraction via extended GCD inversion mod q_i
- [x] `(Px+Q)/(ax²+bx+c)` → `P/(2a)*log(q) + atan_term`

### Repeated Quadratic
- [x] `1/(ax²+c)²` → rational + atan via recursive integration

## Tests

```
∫ 1/(x(x+1)²) dx      → 1/(x+1) + log(x) - log(x+1)         ✅
∫ 1/(x²(x+1)) dx      → -1/x - log(x) + log(x+1)             ✅
∫ x/(x+1)² dx          → 1/(x+1) + log(x+1)                   ✅
∫ 1/(x(x²+1)) dx       → log(x) - (1/2)log(x²+1)             ✅
∫ x/((x+1)(x²+1)) dx   → -½log(x+1) + ¼log(x²+1) + ½atan(x) ✅
∫ (2x+3)/(x²+4) dx     → log(x²+4) + (3/2)atan(x/2)          ✅
∫ 1/(x²+1)² dx         → x/(2(x²+1)) + atan(x)/2              ✅
```

## Remaining (deferred to S9)
- [ ] Full Lazard-Rioboo-Trager (algebraic number field roots)
- [ ] Repeated quadratic factors (general case)
