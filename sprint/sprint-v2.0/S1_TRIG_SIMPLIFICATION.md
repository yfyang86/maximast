# S1 — Trig Simplification

**Status: ✅ Complete**

**Goal:** Implement core trigonometric identities so expressions like
`sin(x)^2 + cos(x)^2` simplify to `1`.

---

## Implemented

### Pythagorean Identities (simp.rs)
- [x] `sin(x)^2 + cos(x)^2 → 1`
- [x] Detection of Pythagorean pairs in `simplify_plus`
- [x] Numeric sum tracking for mixed `sin²+cos²+constant` terms

### Trig Functions (eval.rs)
- [x] `trigexpand`: `sin(a+b)`, `cos(a+b)`, double angles
- [x] `trigreduce`: `sin(x)*cos(x) → sin(2x)/2`
- [x] `trigsimp`: exhaustive Pythagorean simplification

### Boolean Simplification (simp.rs)
- [x] De Morgan's laws: `not(a and b) → not(a) or not(b)`
- [x] Absorption: `a and (a or b) → a`
- [x] Comparison negation: `not(a > b) → a <= b`

### Integration Table Formulas
- [x] sec²(x), csc²(x), tan²(x), cot²(x)
- [x] sin³, cos³, tan³, cot³, sec³, csc³
- [x] sin⁴, cos⁴
- [x] sec·tan, csc·cot, sech·tanh, csch·coth

### Derivative Rules
- [x] cot, sec, csc, acot, coth, sech, csch, acoth, abs

## Tests

```
sin(x)^2 + cos(x)^2               → 1            ✅
trigexpand(sin(a+b))               → sin(a)*cos(b)+cos(a)*sin(b)  ✅
trigreduce(sin(x)*cos(x))          → sin(2*x)/2   ✅
integrate(sec(x)^2, x)            → tan(x)        ✅
integrate(sin(x)^4, x)            → 3x/8 - sin(2x)/4 + sin(4x)/32  ✅
diff(cot(x), x)                   → -csc²(x)      ✅
diff(acot(x), x)                  → -1/(1+x²)     ✅
```
