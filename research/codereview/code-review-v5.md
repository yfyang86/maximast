# Code Review — V5.x Modules (Opus 4.8)

Review of the V5.0/V5.1 additions on the `v5.0`/`dev` branch.
Focus: correctness bugs. Method: source reading + numerical verification in the REPL.

## Bugs Fixed

### 1. `fibonacci(n)` silent integer overflow — **High**
`fib_fast` computed in `i128` then cast `as i64`, silently truncating for
n ≥ 92. `fibonacci(100)` returned `3736710778780434371` instead of
`354224848179261915075`.
**Fix:** compute with `num::BigInt`, return `Expr::BigInt` when the result
exceeds `i64`, else `Expr::Integer`. (numtheory.rs)

### 2. `realpart`/`imagpart`/`conjugate`/`cabs` don't expand — **Medium**
These called `complex_decompose` directly without expanding, so powers and
products of complex numbers were mis-decomposed. `realpart((1+%i)^2)`
returned `(1+%i)^2` instead of `0`; `imagpart` gave `0` instead of `2`;
`cabs` gave `sqrt((1+%i)^4)` instead of `2`. Only `rectform` expanded first.
**Fix:** added `decompose_expanded()` that expands before decomposing, used
by all five functions. (complex.rs)

### 3. `tellsimp`/`tellsimpafter` were functional no-ops — **Medium**
Rules were pushed to `pattern_state.tellsimp_rules` but never read, so
`tellsimp(foo(0), 42); foo(0)` returned `foo(0)`. The feature silently
did nothing despite returning `done`.
**Fix:** added `apply_tellsimp()` (top-level match only, bounded — no
simplification loops) called from `meval` for `Named` operators.
(pattern.rs, eval.rs)

## Verified Correct (no change)
- `inv_mod` with negative `a` (e.g. `inv_mod(-3,7)=2`) — handles gcd sign
- `power_mod` uses `i128` intermediates — no overflow up to `i64::MAX` modulus
- `jacobi`, `chinese` (CRT), `discriminant`, `resultant`, `content` — spot-checked
- Laplace/ILT and ODE solver formulas — verified in earlier sprints

## Known Limitations (not bugs, documented for users)
- **Pattern matching is structural only** — no associative/commutative (AC)
  matching for `+`/`*`. `defrule(r, a+b, ...)` matches a 2-term sum in order
  only. The "full Maxima-compatible" claim in docs overstates this.
- **`nroots` counts distinct real roots** (standard Sturm), not with
  multiplicity: `nroots((x-1)^2, 0, 2)` → 1.
- **`ifactors`/`totient`** use trial division — correct but slow for large
  semiprimes.
- **Minor dead code:** `gcd_u64` in poly_analysis.rs; unused `var_id` guard
  in the `nroots` arm.

## Result
3 correctness bugs fixed, 18 regression tests added.
938 tests pass, 29 walkthroughs pass, zero failures.
