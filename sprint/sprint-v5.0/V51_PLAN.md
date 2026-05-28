# V5.1+ Sprint Plan — Post-V5.0 Continuation

## Current State (V5.0)
920 tests, 250+ functions, 21 walkthroughs. All V5.0 sprints (S1-S8) complete.

## Gap Analysis (tested 2026-05-27)

### Broken / Wrong Output
| Input | Current | Expected | Root Cause |
|-------|---------|----------|------------|
| `%i^2` | `%i^2` | `-1` | No %i simplification |
| `expand((1+%i)^2)` | `1+2*%i+%i^2` | `2*%i` | Same |
| `sin(%pi/6)` | `sin((1/6)*%pi)` | `1/2` | No trig special-value table |
| `cos(%pi/4)` | `cos((1/4)*%pi)` | `sqrt(2)/2` | Same |
| `cos(%pi/2)` | `cos((1/2)*%pi)` | `0` | Same |
| `tan(%pi/4)` | `tan((1/4)*%pi)` | `1` | Same |
| `matrix([1,2],[3,4]) + matrix([5,6],[7,8])` | noun form | `matrix([6,8],[10,12])` | No matrix +/- |
| `3*matrix([1,2],[3,4])` | noun form | `matrix([3,6],[9,12])` | No scalar*matrix |
| `realpart(3+4*%i)` | noun form | `3` | No complex decomp |
| `conjugate(3+4*%i)` | noun form | `3-4*%i` | Same |
| `1/x` display | `x^(-1)` | `1/x` | Display preference |

### Returns Noun Form (not implemented)
| Input | Expected | Difficulty |
|-------|----------|------------|
| `integrate(1/((x^2+4)*(x^2+9)),x,minf,inf)` | `%pi/60` | Medium |
| `ode2('diff(y,x,2)+y=sin(x), y, x)` | particular + homogeneous | Medium |
| `powerseries(exp(x), x, 0)` | `sum(x^n/n!, n, 0, inf)` | Hard |
| `matchdeclare` / `defrule` | pattern matching system | Hard |

### Works Correctly (confirmed)
`sin(%pi)→0`, `exp(log(x))→x`, `log(exp(x))→x`, `ratsimp(sqrt(2)^2)→2`,
`radcan(sqrt(x^2))→abs(x)`, `matrix.matrix` (dot product works),
`errcatch(1/0)→[und]`, `integrate(exp(-x^2),x,minf,inf)→sqrt(%pi)`,
all first-order ODEs, homogeneous second-order ODEs.

---

## Proposed Sprints

### Phase 1: Quick Wins (~6 hours total)

| Sprint | Content | Size | Difficulty |
|--------|---------|------|------------|
| **S9** | Complex number arithmetic (%i^2→-1, realpart, imagpart, conjugate, rectform, cabs) | Small | Easy |
| **S10** | Trig special values (sin(%pi/6)→1/2, full table at multiples of %pi/6, %pi/4) | Small | Easy |
| **S11** | Matrix element-wise arithmetic (+, -, scalar*, negate) | Small | Easy |
| **S12** | Display: `1/x` not `x^(-1)`, fraction form for products with negative exponents | Small | Easy |

### Phase 2: Algorithmic (~8 hours total)

| Sprint | Content | Size | Difficulty |
|--------|---------|------|------------|
| **S13** | Partial fractions for irreducible quadratics → fixes ∫1/((x²+a)(x²+b)) | Medium | Medium |
| **S14** | Non-homogeneous ODE: undetermined coefficients for sin/cos/exp/poly forcing | Medium | Medium |
| **S15** | Sturm chains for `nroots(poly, lo, hi)` and `realroots(poly)` | Medium | Medium |

### Phase 3: Major Subsystems (need design decisions)

| Sprint | Content | Size | Difficulty |
|--------|---------|------|------------|
| **S16** | Pattern matching (defrule/tellsimp) | Large | High |
| **S17** | Arbitrary-precision float (bfloat) | Large | High |
| **S18** | Plotting (gnuplot or native) | Large | Medium |

---

## Sprint Details

### S9: Complex Numbers (Small, ~2h)
**Simplifier changes:**
- `%i^2 → -1`, `%i^3 → -%i`, `%i^4 → 1` (cyclic mod 4)
- `%i^n` for negative n: `%i^(-1) → -%i`, etc.
- Products containing %i: collect real and imaginary parts

**New functions:**
- `realpart(a+b*%i)` → `a` (walk sum, partition by %i content)
- `imagpart(a+b*%i)` → `b`
- `conjugate(a+b*%i)` → `a-b*%i`
- `rectform(expr)` → expand and collect into `a+b*%i`
- `cabs(a+b*%i)` → `sqrt(a²+b²)`

**Core utility:** `complex_decompose(expr) → (real_part, imag_part)` that
recursively walks MPlus/MTimes trees applying `%i² = -1`.

### S10: Trig Special Values (Small, ~1.5h)
Lookup table for `sin`, `cos`, `tan` at rational multiples of `%pi`:

```
k*%pi/6:  k=0 → 0, k=1 → 1/2, k=2 → √3/2, k=3 → 1, ...
k*%pi/4:  k=0 → 0, k=1 → √2/2, k=2 → 1, ...
```

Also: `atan(0) → 0`, `atan(1) → %pi/4`, `asin(1) → %pi/2`, etc.

Reduction rules: `sin(x + n*%pi)`, `sin(-x)`, `cos(x + %pi/2) = -sin(x)`.

### S11: Matrix Element-wise (Small, ~2h)
In `eval_plus` and `eval_times` (or simplifier), detect when both
operands are `MMatrix` with matching dimensions:
- `A + B`: zip rows, zip elements, add pairwise
- `c * A`: map scalar multiply over all elements
- `-A`: negate all elements

### S12: Display Fraction Form (Small, ~1h)
In `Display` for `MTimes`, detect patterns like `a * x^(-n)` and
render as `a/x^n`. Special cases:
- Single `x^(-1)` → `1/x`
- `a * x^(-1)` → `a/x`
- `a * x^(-n)` → `a/x^n`

### S13: Partfrac Irreducible Quadratics (Medium, ~3h)
For `P(x)/((x²+a)(x²+b))`, decompose as:
`(Ax+B)/(x²+a) + (Cx+D)/(x²+b)`
Solve the system for A,B,C,D by coefficient matching.
Then integrate each quadratic factor via completing-the-square → atan/log.

### S14: Non-Homogeneous ODE (Medium, ~3h)
Method of undetermined coefficients for constant-coefficient ODEs with
polynomial, exponential, or trig forcing:
- `y'' + ay' + by = poly(x)` → try polynomial ansatz
- `y'' + ay' + by = exp(cx)` → try `A*exp(cx)`
- `y'' + ay' + by = sin(wx)` → try `A*cos(wx) + B*sin(wx)`
- Resonance case: multiply ansatz by x when forcing overlaps homogeneous

### S15: Sturm Chains (Medium, ~2h)
`nroots(poly, lo, hi)` — count real roots via Sturm sequence:
1. Build Sturm chain: `f₀ = p, f₁ = p', fₖ₊₁ = -rem(fₖ₋₁, fₖ)`
2. Count sign changes at `lo` and `hi`
3. `nroots = V(lo) - V(hi)` where V is sign variation count

`realroots(poly)` — isolate all real roots via bisection after Sturm counting.

---

## Topics Needing Your Input

| # | Topic | Question | Help Needed |
|---|-------|----------|-------------|
| 1 | **Pattern matching scope** | Maxima-compatible `defrule`/`tellsimp`/`tellsimpafter` or a simplified Rust-native rule system? The full Maxima pattern matcher uses `matchdeclare` predicates and is complex. A simpler approach: `defrule(name, pattern, replacement)` with positional wildcards. | Design decision |
| 2 | **bfloat library** | `rug` crate (MPFR wrapper, GPL, fast) vs `dashu` (pure Rust, Apache, portable) vs `astro-float` (pure Rust, MIT)? | Library choice |
| 3 | **Plotting backend** | gnuplot (shell out, well-tested) vs `plotters` crate (native Rust, SVG/PNG, no external dependency)? | Design decision |
| 4 | **Complex number depth** | Just `a+b*%i` decomposition (covers 90% of use cases), or full Gaussian integer ring with GCD and factoring? | Scope decision |
| 5 | **Euler substitution** | The correct formula for `∫1/((x+a)√(x²+c))` — all 3 cases were wrong and removed. Derive together, or defer until a general Euler engine? | Math decision |
| 6 | **Non-homogeneous ODE tolerance** | Undetermined coefficients covers poly/exp/trig forcing. Variation of parameters is more general but needs two antiderivatives (may fail). Acceptable to use undetermined coefficients only and return noun form for other forcing? | Scope decision |
| 7 | **Multivariate polynomials** | The poly crate is univariate. `discriminant(a*x²+b*x+c, x)` returns noun form because `a`, `b`, `c` can't be polynomial coefficients. Worth extending to multivariate, or accept the limitation? | Architecture decision |


---

## Design Decisions (resolved 2026-05-27)

| # | Topic | Decision |
|---|-------|----------|
| 1 | Pattern matching | Full Maxima-compatible (matchdeclare, defrule, tellsimp, tellsimpafter) |
| 2 | bfloat library | Pure Rust first (dashu/astro-float); rug as fallback if features inadequate |
| 3 | Plotting | plotters first (native SVG/PNG), plus gnuplot script export for GNU Maxima compatibility |
| 4 | Complex numbers | Full Gaussian integer ring |
| 5 | Euler substitution | Derive correct formulas together (deferred) |
| 6 | Non-homog ODE | Undetermined coefficients; return noun form with warnings for unsupported forcing |
| 7 | Multivariate poly | Extend to symbolic coefficients (advanced feature, later sprint) |
