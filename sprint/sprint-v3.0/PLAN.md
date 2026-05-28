# Maxima Rust Kernel v3.0 — Algebraic & Advanced Algorithms

## Motivation

v2.0 covers integration/limits/summation for **rational and transcendental**
(log/exp) functions over Q(x). v3.0 tackles the remaining hard-core problems:

1. **Algebraic extensions** — radicals like `sqrt(x²+1)`, `x^(1/3)`, nested roots
2. **Full Gruntz** — the series composition step that handles the PhD-level examples
3. **Zeilberger creative telescoping** — definite hypergeometric summation
4. **Full Lazard-Rioboo-Trager** — logarithmic integration over algebraic number fields
5. **Infrastructure** — hash-consed expressions, performance optimization

These are the mathematically deepest parts of a CAS. Each requires
non-trivial algebraic machinery that doesn't exist in the codebase yet.

---

## Architecture: What v3.0 Requires That v2.0 Doesn't Have

### Algebraic Number Fields
v2.0 works over `Q` (rationals). v3.0 needs `Q(α)` where `α` satisfies
an irreducible polynomial: `α² - 2 = 0` (so `α = √2`), or `α³ - 2 = 0`.

This requires:
- **Minimal polynomial representation**: elements of `Q(α)` are polynomials in `α` modulo `minpoly(α)`
- **Field arithmetic**: add/mul via polynomial arithmetic mod `minpoly`; inverse via extended GCD
- **Tower of algebraic extensions**: `Q ⊂ Q(√2) ⊂ Q(√2, √3)`
- **Norm and trace maps**: needed for resultant computation over extensions

### Recursive Polynomial Representation
v2.0's `Poly` is univariate over `Coeff ∈ {Int, Rat}`. v3.0 needs:
- `Poly<R>` where `R` can be `Coeff`, `AlgExt`, or `Poly<R>` itself (recursive)
- This enables multivariate polynomials as `Poly<Poly<Coeff>>` = polynomials in y with coefficients that are polynomials in x
- Required for Trager's algorithm and Zeilberger

### Formal Power Series (Lazy)
v2.0's `Series` is a truncated vector. v3.0 needs:
- Lazy/streaming series for Gruntz rewrite composition
- `series_compose(f, g)`: substitute series `g` into series `f`
- Handles `exp(series)`, `log(1+series)`, `(1+series)^α` compositions

---

## Sprints

| Sprint | Content | Effort | Depends On |
|--------|---------|--------|------------|
| **V3.1** | Algebraic number field type | 2 weeks | — |
| **V3.2** | Full Gruntz series composition | 1 week | V3.1 (for constant field zero test) |
| **V3.3** | Lazard-Rioboo-Trager over Q̄ | 2 weeks | V3.1 |
| **V3.4** | Trager's algorithm (radical integration) | 3 weeks | V3.1, V3.3 |
| **V3.5** | Zeilberger creative telescoping | 2 weeks | — |
| **V3.6** | Cauchy PV + Almkvist-Zeilberger | 1 week | V3.5 |
| **V3.7** | Hash-consed DAG + optimization | 2 weeks | — |

**Total: ~13 weeks**

---

## V3.1 — Algebraic Number Field Type (2 weeks)

### Goal
Implement `Q(α)` arithmetic where `α` is an algebraic number defined by
its minimal polynomial over `Q`.

### Data Structures

```rust
/// Element of Q(α) where minpoly(α) = 0.
/// Represented as polynomial in α of degree < deg(minpoly).
pub struct AlgNumber {
    pub coeffs: Vec<Rational>,   // coefficients [a₀, a₁, ..., a_{d-1}]
    pub field: AlgField,
}

/// The algebraic number field Q(α).
pub struct AlgField {
    pub minpoly: Vec<Rational>,  // minimal polynomial of α
    pub name: String,            // display name ("sqrt(2)", "α", etc.)
}
```

### Operations
- `AlgNumber::add/sub/mul`: polynomial arithmetic mod `minpoly`
- `AlgNumber::inv`: extended GCD of `coeffs` and `minpoly` → Bezout identity → inverse
- `AlgNumber::norm`: product of conjugates, computed via resultant
- `AlgNumber::trace`: sum of conjugates
- `AlgField::from_sqrt(n)`: create Q(√n) with minpoly x²-n
- `AlgField::from_root(poly)`: create Q(α) with minpoly = poly

### Why This Is Hard
- **Factoring over Q(α)**: to factor polynomials over an algebraic extension,
  need Trager's norm-based method or Lenstra's algorithm
- **GCD over Q(α)**: subresultant GCD needs to work with AlgNumber coefficients
- **Tower extensions**: Q(√2, √3) needs careful representation as Q(√2)(√3)
  with the second minpoly being `x² - 3` but viewed over Q(√2), not Q

### Tests
```
√2 * √2 = 2
(1 + √2) * (1 - √2) = -1
1/(1 + √2) = √2 - 1
minpoly(√2 + √3) = x⁴ - 10x² + 1
```

### Files
- New: `crates/poly/src/alg_field.rs`
- Modify: `crates/poly/src/lib.rs`, `crates/poly/src/gcd.rs` (generic over coefficient ring)

---

## V3.2 — Full Gruntz Series Composition (1 week)

### Goal
Implement `series_compose(outer, inner)` so the MRV rewrite step handles
nested exp/log correctly.

### The Problem
The classic Gruntz example: `limit(exp(x + exp(-x)) - exp(x), x, inf) = 1`

Current v2.0 has partial series expansion for `exp(c*g + small)` but cannot
compose arbitrary series. The rewrite step needs:
```
exp(x + exp(-x)) = exp(x) * exp(exp(-x))
                  = ω⁻¹ * exp(ω)           [where ω = exp(-x)]
                  = ω⁻¹ * (1 + ω + ω²/2 + ...)
                  = ω⁻¹ + 1 + ω/2 + ...
```
Then: `exp(x+exp(-x)) - exp(x) = (ω⁻¹ + 1 + ω/2) - ω⁻¹ = 1 + ω/2 + ...`
Leading term: exponent 0, coefficient 1. Limit = 1.

### Implementation
```rust
/// Compose series: compute f(g(x)) as a series.
/// f is a known function (exp, log, sqrt, etc.)
/// g is a series in ω with g(0) = 0 (or decomposed as g₀ + g̃ with g̃ → 0).
fn series_compose_exp(g: &[(f64, Expr)], order: usize) -> Vec<(f64, Expr)>
fn series_compose_log(g: &[(f64, Expr)], order: usize) -> Vec<(f64, Expr)>
fn series_compose_sqrt(g: &[(f64, Expr)], order: usize) -> Vec<(f64, Expr)>
```

Key algorithm: for `exp(g)` where `g = Σ cᵢ·ω^eᵢ`:
1. Separate constant part: `g = g₀ + g̃` where `g₀` = terms with `eᵢ ≤ 0`
2. `exp(g) = exp(g₀) * exp(g̃)`
3. `exp(g̃) = 1 + g̃ + g̃²/2 + ...` (Taylor, since g̃ → 0)
4. Multiply the ω-series termwise

### Constant-Field Zero Test
Required for reliable sign determination in recursive Gruntz. Given an expression
involving algebraic numbers, determine if it equals zero.
- For Q: exact rational arithmetic (already done)
- For Q(α): compute norm; if norm ≠ 0, the element ≠ 0
- General: Richardson's theorem says this is undecidable, but
  for elementary functions with algebraic coefficients it's decidable

### Tests
```
limit(exp(x + exp(-x)) - exp(x), x, inf) = 1       [THE classic]
limit(exp(exp(x-exp(-x))/(1-1/x)) - exp(exp(x)), x, inf)  [harder]
```

### Files
- Extend: `crates/eval/src/series.rs` (composition functions)
- Extend: `crates/eval/src/gruntz.rs` (use composition in rewrite)

---

## V3.3 — Full Lazard-Rioboo-Trager (2 weeks)

### Goal
Implement the logarithmic part of rational integration for denominators
with algebraic roots.

### The Problem
For `∫ 1/(x³+1) dx`, the denominator factors as `(x+1)(x²-x+1)`.
The quadratic `x²-x+1` has roots `(1±i√3)/2` — complex algebraic numbers.
LRT computes:
1. `R(t) = resultant_x(1 - t·(3x²), x³+1)` — polynomial in t
2. Factor R(t) to find roots (which may be in Q̄, not Q)
3. For each root `cᵢ`: `vᵢ = gcd(1 - cᵢ·3x², x³+1)`
4. Result: `Σ cᵢ · log(vᵢ)`

When roots are complex conjugates, the log terms combine into real
atan expressions via: `c·log(v) + c̄·log(v̄) = Re(c)·log|v|² + 2·Im(c)·arg(v)`

### Implementation
```rust
fn lazard_rioboo_trager(p: &Poly, q: &Poly) -> Vec<(AlgNumber, Poly)>
```

### Requires
- V3.1 (AlgField) for representing algebraic roots of the resultant
- GCD computation with AlgNumber coefficients
- Complex conjugate detection and real/imaginary part extraction

### Tests
```
∫ 1/(x³+1) dx       → -log(x+1)/3 + log(x²-x+1)/6 + atan((2x-1)/√3)/√3
∫ 1/(x⁴+1) dx       → involves √2 in log/atan terms
∫ x²/(x⁴+x²+1) dx  → involves √3
```

### Files
- New/extend: `crates/poly/src/hermite.rs` (LRT function)
- Extend: `crates/eval/src/eval.rs` (wire into integration pipeline)

---

## V3.4 — Trager's Algorithm for Algebraic Function Integration (3 weeks)

### Goal
Integrate functions involving radicals: `∫ f(x, √g(x)) dx`.

### The Problem
```
∫ 1/√(x²+1) dx = asinh(x)
∫ √(1-x²) dx = x√(1-x²)/2 + asin(x)/2
∫ 1/(x·√(x²-1)) dx = acos(1/x)
∫ x/√(x⁴+1) dx = (1/2)·asinh(x²)
```

These are **algebraic functions** — `√g(x)` satisfies `t² - g(x) = 0`.
The Risch algorithm extends to handle these via:
1. Represent `√g` as an algebraic extension: `t` where `t² = g(x)`
2. The integrand lives in `Q(x)[t]/(t² - g)` — a two-dimensional algebra over Q(x)
3. Apply Hermite reduction in this algebra
4. Logarithmic part via Trager's variant of LRT over the algebraic extension

### Implementation
```rust
enum AlgExtension {
    Sqrt(Expr),              // t² = expr
    Root(Poly, u32),         // t^n = expr (nth root)
}

fn integrate_algebraic(
    f: &Expr, var: &Expr, ext: &AlgExtension
) -> Option<Expr>
```

### Why This Is the Hardest Part
1. **Representation**: elements of `Q(x)(√g)` are `a(x) + b(x)·√g` where a,b are rational functions
2. **Derivative**: `d/dx[√g] = g'/(2√g)` — the derivative involves the algebraic element itself
3. **GCD in the extension**: need polynomial GCD over `Q(x)[t]/(t²-g)`, which requires norm-based techniques
4. **Hermite reduction over the extension**: the denominator is a polynomial in `x` and `t`, factored over the algebraic closure
5. **Logarithmic part**: Trager's method computes resultants and GCDs over the extension field

### Euler Substitutions (Alternative)
For `∫ R(x, √(ax²+bx+c)) dx`, Euler substitutions transform to rational:
- If `a > 0`: let `√(ax²+bx+c) = t - x√a`
- If `c > 0`: let `√(ax²+bx+c) = xt + √c`
- Otherwise: let `√(a(x-r₁)(x-r₂)) = t(x-r₁)` where r₁ is a root

This transforms the algebraic integral into a rational one over Q(t),
which v2.0's Hermite+partfrac can handle. Simpler than full Trager.

### Tests
```
∫ 1/√(x²+1) dx                    → asinh(x)
∫ 1/√(1-x²) dx                    → asin(x)
∫ √(1-x²) dx                      → x√(1-x²)/2 + asin(x)/2
∫ 1/(x·√(x²-1)) dx                → acos(1/x)
∫ (2x+1)/√(x²+x+1) dx            → 2√(x²+x+1)
∫ √(x²+2x+2) dx                   → completing square + standard form
```

### Files
- New: `crates/eval/src/risch_algebraic.rs`
- Extend: `crates/eval/src/risch_tower.rs` (Algebraic extension variant)
- Extend: `crates/poly/src/` (polynomial GCD over extensions)

---

## V3.5 — Zeilberger Creative Telescoping (2 weeks)

### Goal
Given `F(n,k)` hypergeometric in both `n` and `k`, find a recurrence
for `S(n) = Σ_k F(n,k)`.

### The Problem
```
Σ_{k=0}^{n} binomial(n,k) = 2^n
Σ_{k=0}^{n} binomial(n,k)² = binomial(2n,n)
Σ_{k=0}^{n} (-1)^k·binomial(n,k)·binomial(2k,k)·4^(n-k) = binomial(2n,n)
```

Zeilberger's algorithm finds a linear recurrence `a₀(n)S(n) + a₁(n)S(n+1) + ... = 0`
by searching for a "certificate" `R(n,k)` such that:
`Σⱼ aⱼ(n)·F(n+j,k) = G(n,k+1) - G(n,k)` where `G = R·F` (telescoping)

### Algorithm (A=B, Chapter 6)
```
zeilberger(F, n, k, order):
  for J = 1, 2, ...:
    # Try to find certificate of order J
    # Set up: a₀·F(n,k) + a₁·F(n+1,k) + ... + aJ·F(n+J,k) = ΔG
    # where G = R(n,k)·F(n,k) and ΔG = G(n,k+1)-G(n,k)
    # Compute F(n+j,k)/F(n,k) = rational in (n,k)
    # Apply Gosper to the resulting sum → solve for R and the aⱼ
    if gosper_succeeds:
      return recurrence [a₀, ..., aJ]
  return None  # no recurrence of bounded order
```

### Requires
- Gosper's algorithm (done in v2.0)
- Polynomial arithmetic in TWO variables (n and k)
- Rational function arithmetic in two variables

### Implementation
```rust
fn zeilberger(
    f: &Expr,        // F(n,k) as expression
    n: &Expr,        // parameter variable
    k: &Expr,        // summation variable
    max_order: u32,  // maximum recurrence order to try
) -> Option<Vec<Expr>>  // recurrence coefficients [a₀(n), a₁(n), ...]
```

### Tests
```
zeilberger(binomial(n,k), n, k) → [1, -2] (S(n+1) = 2·S(n))
sum(binomial(n,k), k, 0, n) → 2^n (via solving the recurrence)
```

### Files
- Extend: `crates/eval/src/eval.rs` (summation section)
- May need: `crates/poly/src/bivariate.rs` (two-variable polynomial type)

---

## V3.6 — Cauchy PV + Almkvist-Zeilberger (1 week)

### Cauchy Principal Value
```
PV ∫_{-1}^{1} 1/x dx = 0
PV ∫_{-∞}^{∞} 1/(x(x²+1)) dx = 0
```

Detect real-axis poles, split integral symmetrically, take limits.

### Almkvist-Zeilberger
Continuous analogue of Zeilberger for parametric integrals:
`I(n) = ∫ F(n,x) dx` → find recurrence in n.

Example: `∫_0^∞ x^n·exp(-x) dx = n!` satisfies `I(n+1) = (n+1)·I(n)`.

### Files
- Extend: `crates/eval/src/eval.rs`

---

## V3.7 — Hash-Consed DAG + Performance (2 weeks)

### Goal
Replace the tree-based `Expr` with a hash-consed directed acyclic graph
for O(1) structural equality and sharing of common subexpressions.

### Why
- Currently, `expr == expr` requires full tree traversal: O(n)
- Common subexpressions are duplicated in memory
- The simplifier repeatedly reconstructs identical nodes
- With hash-consing: equality is pointer comparison, sharing is automatic

### Implementation
```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ExprId(u32);  // index into global arena

pub struct ExprArena {
    nodes: Vec<ExprNode>,
    intern: HashMap<ExprNode, ExprId>,
}

enum ExprNode {
    Int(i64),
    Rat(i64, i64),
    Sym(SymbolId),
    App(SymbolId, Vec<ExprId>),  // function application
}
```

### Impact
- **Major refactor**: every file that uses `Expr` changes
- **Performance**: potentially 5-10x for simplification-heavy workloads
- **Memory**: reduced via sharing
- **Correctness**: O(1) equality enables better caching of simplification results

### Risk
This touches the entire codebase. Should be done on a separate branch
with careful migration and extensive testing.

---

## Dependencies

```
V3.1 (AlgField) ─────→ V3.3 (LRT over Q̄) ─→ V3.4 (Trager)
       │
       └──→ V3.2 (Series composition, zero test)

V3.5 (Zeilberger) ──→ V3.6 (Cauchy PV, A-Z)

V3.7 (Hash-consed DAG) — independent, risky, do last
```

## Success Metrics

| Metric | v2.0 | v3.0 Target |
|--------|------|-------------|
| Integration formulas | 59/59 (100%) | + 30 radical/algebraic formulas |
| rtest1 | 164/208 (79%) | 180+ (87%+) |
| Gruntz PhD examples | partial | all standard examples |
| `∫ 1/√(x²+1)` | noun form | `asinh(x)` |
| `∫ √(1-x²)` | noun form | `x√(1-x²)/2 + asin(x)/2` |
| `∫ 1/(x³+1)` | partial | full log+atan via LRT |
| `limit(exp(x+exp(-x))-exp(x))` | `und` | `1` |
| `Σ binomial(n,k)²` | noun form | `binomial(2n,n)` via Zeilberger |
| Total tests | 655 | 800+ |

## References

1. Bronstein, *Symbolic Integration I — Transcendental Functions*, Springer 2005 (ch. 11-12 for algebraic)
2. Trager, *Integration of Algebraic Functions*, MIT PhD 1984
3. Petkovšek, Wilf, Zeilberger, *A=B*, A K Peters 1996 (ch. 6-7 for Zeilberger)
4. Gruntz, *On Computing Limits in a Symbolic Manipulation System*, ETH PhD 1996
5. Cohen, *A Course in Computational Algebraic Number Theory*, Springer 1993
6. FriCAS source: `intalg.spad` (algebraic integration), `zeilberg.spad` (Zeilberger)
7. SymPy source: `risch.py`, `integrals/trigonometry.py`
