# Risch + Gruntz Implementation Notes

## Status: ✅ Core implementation complete

---

## Architecture

### Integration Pipeline (eval.rs: table_integrate)

```
integrate(f, x):
  1. Risch-Norman heuristic (risch_norman.rs)     — fast ansatz path
  2. Pattern matching (table_integrate)             — 55+ formulas
     a. Constant, variable, sum, product linearity
     b. Power rule, named functions (trig, hyp, inv-trig)
     c. Function powers (sin², cos², sec², etc. up to n=4)
     d. Linear substitution f(ax) → F(ax)/a
     e. Integration by parts (x*f(x), x^n*log(x), x*exp(ax))
  3. Fraction recognition                          — 1/sqrt, 1/(ax²+bx+c)
  4. Derivative recognition                        — f'/f → log(f)
  5. Rational function pipeline                    — polynomial division
     a. GCD cancellation
     b. Polynomial division (extract polynomial part)
     c. Factor denominator (sqfree + rational roots + Kronecker)
     d. Repeated factors → Hermite reduction
     e. Single linear factor → log or power
     f. Multiple linear factors → partial fractions → log
     g. Mixed linear+quadratic → integrate_partfrac_mixed
  6. Substitution engine (try_substitution_integrate)
     a. Power substitution: f = c*u^n*u' → c*u^(n+1)/(n+1)
     b. Factor partitioning with numeric scale detection
     c. Power rewriting: x^(kn) → u^k when u=x^n
  7. Risch tower integration (risch_integrate.rs)
     a. Tower construction (risch_tower.rs)
     b. Primitive (log) case
     c. Exponential case with Risch DE solver
  8. Noun form fallback
```

### Limit Pipeline (gruntz.rs + eval.rs)

```
limit(f, x, point):
  For x → ±∞:
    1. Polynomial/rational degree analysis
    2. gruntz_limit:
       a. gruntz_mrv (proper algorithm):
          - Compute MRV set
          - Choose ω from MRV (exp or var)
          - Rewrite f as series in ω → 0
          - Leading term analysis + recursion
       b. limitinf (heuristic fallback):
          - Growth order classification
          - Dominant term selection in sums
          - Exponential dominance in products
          - 0*∞ handler (sin(t)/t → 1 etc.)
          - 1^∞ handler (log(1+f) ≈ f)

  For x → finite a:
    1. Direct substitution
    2. Rational function: num/den degree comparison
    3. 0/0: iterated L'Hôpital (up to 5 times)
```

### Summation Pipeline (eval.rs: try_closed_form_sum)

```
sum(body, k, lo, hi):
  1. Numeric bounds (< 10000): iterate
  2. Polynomial in k: Faulhaber formulas (Σk, Σk², Σk³)
  3. Geometric: Σ c*r^k → formula
  4. Telescoping: detect f(k)-f(k+1), compute f(lo)-f(hi+1)
  5. Partial fractions → telescoping: 1/(k(k+1)) etc.
  6. Arith-geometric: Σ k*r^k → formula
  7. Gosper: hypergeometric ratio test, certificate with verification
  8. Noun form fallback
```

---

## Key Data Structures

### Series (eval/series.rs)
```rust
struct Series {
    terms: Vec<(i64, i64, Expr)>,  // (exp_num, exp_den, coefficient)
    var: Expr, center: Expr, order: i64,
}
```

### Tower (eval/risch_tower.rs)
```rust
enum Extension { Primitive { log_arg: Expr }, Exponential { exp_arg: Expr } }
struct Tower {
    var: Expr,
    extensions: Vec<(SymbolId, Extension, Expr)>,  // (var, type, derivative)
}
```

### CRE (poly/cre.rs)
```rust
struct CRE { num: Poly, den: Poly, var: SymbolId }
// Always in lowest terms; operations maintain normalization
```

---

## Files Modified/Created

| File | Lines | Change |
|------|-------|--------|
| `eval/src/eval.rs` | 7800+ | Major: integration pipeline, definite integrals, summation |
| `eval/src/gruntz.rs` | 600 | Rewritten: MRV + heuristic |
| `eval/src/series.rs` | 257 | New: truncated power series |
| `eval/src/risch_tower.rs` | 193 | New: differential field tower |
| `eval/src/risch_integrate.rs` | 355 | New: Risch integration |
| `eval/tests/formula_test.rs` | 169 | New: 185-formula test suite |
| `poly/src/gcd.rs` | +60 | Fix: rational coefficient normalization |
| `poly/src/convert.rs` | +55 | New: CRE ↔ Expr conversion |
| `poly/src/factor.rs` | +14 | Fix: factoring test |
| `poly/src/cre.rs` | 224 | Existing: CRE arithmetic |
| `poly/src/traits.rs` | 150 | Existing: trait hierarchy |

---

## References

1. Bronstein, *Symbolic Integration I*, Springer 2005
2. Gruntz, *On Computing Limits in a Symbolic Manipulation System*, ETH 1996
3. Geddes, Czapor, Labahn, *Algorithms for Computer Algebra*, Kluwer 1992
4. Petkovšek, Wilf, Zeilberger, *A=B*, A K Peters 1996
