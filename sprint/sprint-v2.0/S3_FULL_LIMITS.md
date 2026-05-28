# S3/S6 — Full Gruntz Limits + Series Engine

**Status: ✅ Complete** (originally S3, expanded into S6 in revised plan)

**Goal:** Replace the growth-order heuristic with the proper Gruntz
algorithm using MRV sets and series expansion.

---

## Implemented

### Series Type (eval/series.rs — 257 lines)
- [x] `Series` struct: `Vec<(i64, i64, Expr)>` — (num_exp, den_exp, coefficient)
- [x] `add`, `mul` with convolution and like-term combining
- [x] `leading_term`, `leading_exponent`, `leading_coeff`
- [x] `to_expr` — reconstruct polynomial expression
- [x] `series_at_zero` for exp, sin, cos, log(1+x)
- [x] `taylor` via repeated differentiation

### MRV Gruntz Algorithm (eval/gruntz.rs — 600 lines)
- [x] `compute_mrv(expr, var)` — find most rapidly varying subexpressions
- [x] `mrv_max` — merge MRV sets keeping fastest-growing
- [x] `choose_omega` — select representative from MRV
- [x] `rewrite_exp_omega` — express `f` as series in `ω = exp(-g) → 0`
- [x] `rewrite_var_omega` — handle `ω = x` (polynomial/log case)
- [x] `gruntz_mrv` — main algorithm: MRV → ω → rewrite → leading term → recurse
- [x] Falls back to heuristic `limitinf` when MRV approach fails

### Series Expansion in Rewrite
- [x] `exp(c*g + small)` → `ω^(-c) * (1 + small + small²/2 + ...)`
- [x] Decomposition of exp argument into g-proportional + remainder
- [x] Second-order Taylor for `exp(small)` when `small → 0`
- [x] Term combination and cancellation after rewriting

### Growth Order Fixes
- [x] `exp(-x)` correctly identified as decay (check sign of argument)
- [x] `inf^(-n) → 0`, `inf^(+n) → inf`
- [x] `f^(-n) → 0` when `f → ∞` (including `log(x)^(-1)`)
- [x] Exponential dominance: `exp(x)*poly → ∞` regardless of poly degree

### Indeterminate Form Handlers
- [x] `0*∞`: detect `f(t)*g` where `t→0`, `g→∞`, `f(t)/t → c`
- [x] Known limits: `sin(t)/t → 1`, `tan(t)/t → 1`, etc.
- [x] Log-growth `0*∞`: `log^n * poly_decay → 0`
- [x] `∞-∞` cancellation: rationalize dominant terms
- [x] `1^∞`: `log(1+f) ≈ f` approximation

### Function Evaluation in Limits
- [x] `sin(0) = 0`, `cos(0) = 1`
- [x] `atan(+∞) = π/2`, `atan(-∞) = -π/2`
- [x] Iterated L'Hôpital (up to 5 times) for finite-point 0/0

## Tests

```
limit(x^2, x, inf)                    → inf          ✅
limit(1/x, x, inf)                    → 0            ✅
limit(exp(-x), x, inf)                → 0            ✅
limit((x^2-1)/(x-1), x, 1)            → 2            ✅ (L'Hôpital)
limit(sin(x)/x, x, 0)                 → 1            ✅
limit((exp(x)-1)/x, x, 0)             → 1            ✅
limit(x*log(x), x, 0)                 → 0            ✅
limit(x^x, x, 0)                      → 1            ✅
limit((1+1/x)^x, x, inf)              → exp(1)       ✅ (1^∞)
limit(exp(x)/x^2, x, inf)             → inf          ✅ (exp dominance)
limit(exp(x)/x^100, x, inf)           → inf          ✅ (MRV)
limit(log(x)/x, x, inf)               → 0            ✅
limit(x*sin(1/x), x, inf)             → 1            ✅ (0*∞)
limit(x*exp(-x), x, inf)              → 0            ✅
limit(exp(exp(x)), x, inf)            → inf          ✅ (nested exp)
limit(log(log(x)), x, inf)            → inf          ✅ (nested log)
limit(log(log(x))/log(x), x, inf)     → 0            ✅ (log ratio)
limit((exp(x)-1-x)/x^2, x, 0)        → 1/2          ✅ (iterated L'Hôpital)
limit(log(x)^10/x, x, inf)            → 0            ✅ (log^n/poly)
```

## Remaining
- [ ] Full recursive series composition in rewrite step
- [ ] `exp(x+exp(-x))-exp(x) → 1` (Gruntz PhD classic — needs deeper series)
- [ ] `sqrt(x²+1)-x → 0` (conjugate rationalization)
