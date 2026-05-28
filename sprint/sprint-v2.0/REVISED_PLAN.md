# v2.0 Revised Sprint Plan

## Status Summary (2026-05-25)

| Sprint | Status | Tasks | Key Deliverable |
|--------|--------|-------|-----------------|
| **S1** Trig simplification | **Complete** | — | Pythagorean, De Morgan, canonical ordering |
| **S2** Traits + CRE | **Complete** | 6/8 | Ring hierarchy, CRE arithmetic, CRE↔Expr |
| **S3** Risch-Norman | **Complete** | 5/5 | Heuristic ansatz integration |
| **S4** Rational integration | **Complete** | 11/13 | Hermite, partfrac (linear+quadratic) |
| **S5** Risch transcendental | **Complete** | 16/17 | Tower, primitive/exp case, Risch DE, substitution |
| **S6** Gruntz + series | **Complete** | 14/16 | Series type, MRV algorithm, exp rewrite |
| **S7** Summation | **Complete** | 9/10 | Faulhaber, geometric, Gosper, telescoping |
| **S8** Definite integration | **Complete** | 11/14 | Infinite bounds, Gaussian, residues |
| **S9** Algebraic (future) | Deferred | 0/3 | Radical extensions |

**Overall: 72/87 tasks done (83%) — all 8 core sprints substantially complete**

### Test Coverage
- **644 total tests**: 404 eval + 106 poly + 78 parser + 43 core + 12 rtest + 1 formula
- **Integration formula suite**: 51/59 non-parametric formulas pass (86.4%)
- **rtest1**: 164/208 (79%)
- **rtest_boolean**: 88/116 (76%)

### New Modules (v2.0)
| Module | Lines | Purpose |
|--------|-------|---------|
| `series.rs` | ~220 | Truncated power series with rational exponents |
| `risch_tower.rs` | ~170 | Differential field tower construction |
| `risch_integrate.rs` | ~280 | Tower-based Risch integration |
| `gruntz.rs` | ~600 | MRV Gruntz algorithm + heuristic fallback |
| `formula_test.rs` | ~140 | Automated integration formula test suite |

---

## Based on Algorithm Survey Research

The survey identifies the **core strategic path**: implement the
differential-algebra layer (Risch, Gruntz, summation) natively in Rust,
using FLINT via FFI for the polynomial/number-theoretic core.

Our v1.0 implementation is a workaround that covers simple cases.
v2.0 must do it properly to be a credible CAS.

---

## Architecture Change: Trait Hierarchy First

Before implementing algorithms, establish a FriCAS-style trait tower.
This is the **single most important structural decision** per the survey.

```rust
trait Ring: Clone + PartialEq {
    fn zero() -> Self; fn one() -> Self;
    fn add(&self, other: &Self) -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn neg(&self) -> Self;
}
trait IntegralDomain: Ring { fn is_zero(&self) -> bool; }
trait GcdDomain: IntegralDomain { fn gcd(&self, other: &Self) -> Self; }
trait EuclideanDomain: GcdDomain { fn divmod(&self, other: &Self) -> (Self, Self); }
trait Field: EuclideanDomain { fn inv(&self) -> Self; }
trait DifferentialRing: Ring { fn deriv(&self, var: &Var) -> Self; }
trait FunctionSpace: DifferentialRing + Field { ... }
```

---

## Revised Sprints

| Sprint | Content | Status | Remaining |
|--------|---------|--------|-----------|
| **S1** | Trig simplification | ✅ Done | — |
| **S2** | Trait hierarchy + CRE | ✅ Done | Hash-consing, benchmarks (deferred) |
| **S3** | Risch-Norman heuristic | ✅ Done | — |
| **S4** | Complete rational integration | ✅ Done | Full LRT, repeated quadratics (→S9) |
| **S5** | Risch transcendental | ✅ Done | Polynomial tower reduction |
| **S6** | Gruntz + series engine | ✅ Done | Full series composition, more PhD examples |
| **S7** | Gosper + Zeilberger | ✅ Done | Zeilberger creative telescoping |
| **S8** | Definite integration | ✅ Done | Higher-order residues, Cauchy PV, A-Z |
| **S9** | Risch algebraic | Deferred | Algebraic extensions, radical integration |
| **S10** | FLINT FFI | Deferred | Polynomial core optimization |

---

## S2: Trait Hierarchy + Polynomial Abstraction (2 weeks)

### Goal
Establish the algebraic type system that all algorithms build on.
This is the "Phase 0" from the survey — do it before anything else.

### Tasks

- [x] Define trait hierarchy: Ring → IntegralDomain → GcdDomain → EuclideanDomain → Field
- [x] Define DifferentialRing trait
- [x] Implement traits for: i64, Coeff
- [x] Canonical Rational Expression type: `CRE { num: Poly, den: Poly }`
- [x] CRE arithmetic (add, sub, mul, div, derivative)
- [x] CRE ↔ Expr conversion: expr_to_cre, cre_to_expr
- [→v4] Hash-consed expression DAG → V4.7
- [→v4] CRE benchmarks → V4.7

### Design: CRE Type

```rust
pub struct CRE {
    pub num: Poly,
    pub den: Poly,
    pub var: SymbolId,
}

impl CRE {
    pub fn new(num: Poly, den: Poly) -> Self { /* GCD normalize */ }
    pub fn add(&self, other: &CRE) -> CRE { /* cross-multiply */ }
    pub fn derivative(&self) -> CRE { /* quotient rule: (n'd-nd')/d² */ }
    pub fn is_zero(&self) -> bool { self.num.is_zero() }
}
```

---

## S3: Risch-Norman Heuristic (1 week)

### Goal
Fast first-try integration that handles ~80% of textbook integrals.
Make an ansatz, differentiate, solve the linear system.

### Algorithm (per Boettner 2010, Geddes/Czapor/Labahn ch. 12)

```
integrate_heuristic(f, x):
1. Guess form: F = Σ cᵢ * Bᵢ(x) where Bᵢ are building blocks
   (polynomials, logs, exponentials of subexpressions in f)
2. Differentiate: F' = Σ cᵢ * Bᵢ'(x)
3. Match: F' = f → linear system in cᵢ
4. Solve system. If solution exists, return F.
5. If no solution, fall through to full Risch.
```

### Tasks

- [x] Extract "building blocks" from integrand (subexpressions, their logs/exps)
- [x] Construct ansatz with undetermined coefficients (degree 0 and 1)
- [x] Differentiate ansatz symbolically
- [x] Match coefficients → solve for constants
- [x] Wire as first-try before table lookup

---

## S4: Complete Rational Integration (2 weeks)

### Goal
Full Hermite + Lazard-Rioboo-Trager. Handle ANY rational function.

### Algorithm (per Bronstein ch. 2, Lazard-Rioboo 1990)

**Hermite Reduction** (done partially in v1.0):
```
∫ p/q dx = rational_part + ∫ reduced_p / sqfree_q dx
```

**Lazard-Rioboo-Trager** (the logarithmic part):
```
∫ p/q dx where q is square-free:
1. R(t) = resultant_x(p - t*q', q)
2. Factor R(t) to find roots cᵢ ∈ Q̄
3. For each cᵢ: vᵢ = gcd(p - cᵢ*q', q)
4. Result = Σ cᵢ * log(vᵢ)
```

The key improvement over our current impl: handle complex roots
properly (conjugate pairs → atan) and repeated quadratic factors.

### Tasks

- [x] Complete Hermite reduction: per-factor iterative reduction for linear factors
- [x] Fix poly_gcd: normalize polynomials with rational coefficients to monic integer form
- [x] Fix sqfree/factor_poly: correct factoring of x(x+1)^2 and similar
- [x] Partial fraction integration for square-free linear denominators
- [x] Full pipeline: polynomial division → Hermite → partfrac → combine
- [x] Tests: 1/(x(x+1)^2), 1/(x^2(x+1)), x/(x+1)^2, 1/(x+1)^3, 1/((x+1)(x+2)^2)
- [x] Implement resultant via Sylvester (done in v1.0 hermite.rs)
- [x] Partial fraction for mixed linear + irreducible quadratic factors
- [x] Quadratic factor → log + atan via (Px+Q)/(ax²+bx+c) decomposition
- [x] Extended GCD inversion modulo quadratic for coefficient extraction
- [x] Tests: 1/(x(x²+1)), x/((x+1)(x²+1)), (2x+3)/(x²+4)
- [x] Lazard-Rioboo-Trager: resultant-based log coefficient extraction via interpolation
- [x] Repeated quadratic factors: ∫ k/(x²+c)^n via iterative reduction

---

## S5: Risch Transcendental Integration (4 weeks)

### Goal
Handle `∫ f(x, log(g), exp(h)) dx` — the transcendental Risch algorithm.

### Algorithm (per Bronstein 2005, chapters 4-7)

The key idea: build a "tower of extensions" over Q(x):
```
Q(x) ⊂ Q(x, θ₁) ⊂ Q(x, θ₁, θ₂) ⊂ ...
where each θᵢ is either log(f) or exp(f) for some f in the previous field.
```

For each extension type:

**Primitive case** (θ = log(f)):
- Hermite reduction in the tower
- Parametric logarithmic derivative problem
- Rothstein-Trager for new log terms

**Exponential case** (θ = exp(f)):
- Risch differential equation: `y' + f'y = g`
- Polynomial part: bound degree, solve coefficient system
- Exponential part: check integrability conditions

### Tasks

- [x] Integration by substitution engine (try_substitution_integrate)
- [x] Structural factor decomposition with distributed negative exponents
- [x] Power substitution: f = c * u^n * u' → c * u^(n+1)/(n+1)
- [x] Factor partitioning: separate u-dependent from var-dependent, check u' proportionality
- [x] Tests: 1/(x*log(x)), log(x)/x, log(x)²/x, x*exp(x²), exp(x)/(1+exp(x))
- [x] x^n*log(x) by-parts formula (#47)
- [x] x*exp(a*x) by-parts formula (#42)
- [x] sin⁴(x), cos⁴(x) power reduction
- [x] 1/(ax²+c)² repeated quadratic → rational + atan
- [x] Power substitution with rewriting: x^(kn) → u^k when u=x^n
- [x] Tests: x²log(x), x*exp(2x), sin⁴(x), 1/(x²+1)², x/(x⁴+1)
- [x] Tower construction: risch_tower.rs with Extension enum, build_tower, rewrite
- [x] Primitive (log) case: polynomial-in-t ansatz, t'*t^n pattern matching
- [x] Exponential case: Laurent decomposition, Risch DE for each exp term
- [x] Risch differential equation solver: B'+f'B=g for polynomial B
- [x] Wired into table_integrate pipeline; tests: 1/(x*log(x)²)=-1/log(x)
- [x] Polynomial reduction in tower: general degree ansatz for log extensions

### Key references
- Bronstein, *Symbolic Integration I*, chapters 4-7
- FriCAS `intef.spad`, `intrf.spad`
- SymbolicIntegration.jl (Julia, October 2025)

---

## S6: Gruntz with Proper Series Engine (2 weeks)

### Goal
Replace the growth_order hack with the actual Gruntz algorithm:
MRV set → rewrite → series expansion → recursive limit.

### Algorithm (per Gruntz PhD 1996, SymPy gruntz.py)

```
limitinf(f, x):
1. Compute MRV(f, x) — most rapidly varying subexpressions
2. Choose ω ∈ MRV with ω = exp(g(x))
3. Rewrite f in terms of ω: f = Σ cᵢ · ω^eᵢ
   (this is the HARD part — requires series expansion)
4. Leading term: (c₀, e₀)
5. If e₀ > 0: limit = 0
   If e₀ < 0: limit = ±∞ (sign of c₀)
   If e₀ = 0: limit = limitinf(c₀, x) [recurse]
```

### Series Engine (minimal)

```rust
pub struct Series {
    terms: Vec<(Rational, Expr)>, // (exponent, coefficient)
    var: SymbolId,
    truncation: u32,
}

impl Series {
    fn from_expr(expr, var, order) -> Series;
    fn leading_term(&self) -> (Rational, Expr);
    fn add/mul/compose operations;
}
```

Key series needed:
- `exp(f) = 1 + f + f²/2 + ...`
- `log(1+f) = f - f²/2 + f³/3 - ...`
- `(1+f)^a = 1 + af + a(a-1)f²/2 + ...`

### Tasks

- [x] Fix growth_order for exp(-x): check sign of argument at ∞
- [x] Exponential dominance: exp(x)*poly(x) → ∞ regardless of poly degree
- [x] 0*∞ handler: detect f(t)*g where t→0, g→∞, f(t)/t → c (sin, tan, etc.)
- [x] Evaluate sin(0)=0, cos(0)=1 in limit handler
- [x] Tests: exp(x)/x², x*sin(1/x), x*exp(-x), (1+1/x)^x, sin(x)/x
- [x] Series type with rational exponents (series.rs module)
- [x] Series construction for exp, sin, cos, log1p
- [x] Series multiplication and addition
- [x] Taylor series via repeated differentiation
- [x] MRV set computation (proper Gruntz algorithm)
- [x] Rewrite step: express f as series in ω = exp(-g) → 0
- [x] Main gruntz_mrv with leading-term extraction + recursion
- [x] Tests: exp(x)/x^100, exp(exp(x)), log(log(x))
- [x] Series expansion in exp rewrite: exp(c*g + small) → ω^(-c)*(1+small+small²/2)
- [x] Tests: log(log(x))/log(x)→0, (exp(x)-1-x)/x²→1/2, log(x)^10/x→0
- [x] Numeric ratio fallback in exp rewrite (enables -x/x → -1)
- [x] sqrt(x²+1)-x → 0 via conjugate rationalization
- [x] exp(x+exp(-x))-exp(x) → 1 via omega selection + series expansion

### Known failure modes (from survey)
- The rewrite step is where bugs live (series of log singularities)
- Constant-field zero test needed (sin²+cos²-1 = 0)
- Oscillatory functions at infinity must be special-cased
- Recursion depth must be bounded

---

## S7: Gosper + Zeilberger Summation (1 week)

### Goal
Closed-form summation of hypergeometric terms + creative telescoping.

### Gosper's Algorithm (per A=B, ch. 5)
```
Given: Σ_{k=0}^{n} t_k where t_{k+1}/t_k = r(k)/s(k)
Find: S_k such that t_k = S_{k+1} - S_k (telescoping)
```

### Zeilberger's Algorithm (per A=B, ch. 6)
```
Given: F(n,k) hypergeometric in both n and k
Find: recurrence p₀(n)·S(n) + p₁(n)·S(n+1) + ... = 0
where S(n) = Σ_k F(n,k)
```

### Tasks

- [x] Closed-form summation engine: try_closed_form_sum
- [x] Faulhaber's formulas: Σk, Σk², Σk³ for polynomial sums
- [x] Geometric series: Σ c*r^k = c*(r^(n+1)-r^lo)/(r-1)
- [x] Telescoping detection: Σ(f(k)-f(k+1)) = f(lo)-f(hi+1)
- [x] Partial fractions for rational sums: 1/(k(k+1)) → telescoping
- [x] Arith-geometric: Σ k*r^k closed form
- [x] Tests: Σk, Σk², Σk³, Σ2^k, Σ1/(k(k+1)), Σk·2^k, harmonic noun
- [x] Gosper's algorithm: ratio test, certificate S_k=t_k*q(k-1)/d with verification
- [x] Hypergeometric term detection: t_{k+1}/t_k rational check
- [x] Zeilberger's algorithm: parametrized Gosper with numeric recurrence detection

---

## S8: Definite Integration (2 weeks)

### Goal
Residue-based definite integration + Almkvist-Zeilberger creative telescoping.

### Tasks

- [x] Definite integration via antiderivative + evaluation at bounds
- [x] Infinite bounds: ∫_a^∞ via Gruntz limit of antiderivative
- [x] Double-infinite bounds: ∫_{-∞}^∞ via both limits
- [x] Known definite integrals: Gaussian ∫exp(-x²)=√π, ∫_0^∞ exp(-ax)=1/a
- [x] Residue method for ∫_{-∞}^∞ c/(ax²+d) = cπ/√(ad)
- [x] atan(±∞) = ±π/2 in limit handler
- [x] Tests: ∫x²[0,1], ∫sin(x)[0,π], ∫exp(-x)[0,∞], Gaussian, ∫1/(x²+1)[-∞,∞]
- [x] General residue computation for irreducible quadratic factors
- [x] Repeated quadratic (x²+c)² residues: π/(2c√(ac))
- [x] Two distinct quadratics: pπ/(√(a₁a₂)(√a₁+√a₂))
- [x] Residue method tried before antiderivative for (-∞,∞) integrals
- [x] Higher-order pole residues: ∫ k/(x²+c)^n via (2n-2)!/(2^(2n-1)(n-1)!²) formula
- [x] Cauchy principal value: odd function detection over symmetric intervals
- [→v4] Almkvist-Zeilberger → V4.5

---

## Dependencies

```
S1 (done) ─────────────────────────────────────────────────
S2 (traits) → S3 (Risch-Norman) → S5 (Risch transcendental)
           → S4 (rational complete) ↗
           → S6 (Gruntz + series)
           → S7 (summation)
           → S8 (definite integrals) ← S4
```

---

## Testing Strategy

### Unit Tests (644 total)
| Crate | Tests | Coverage |
|-------|-------|----------|
| eval | 404 | Integration (55+), limits (15+), sums (12), diff (10+), simplifier, builtins |
| poly | 106 | GCD, factoring, CRE, hermite, groebner, traits |
| parser | 78 | Lexer, parser, all Maxima syntax |
| core | 43 | Expr, operators, interning |
| rtest | 12 | rtest1, boolean, equal, abs, dot, algebraic, etc. |
| formula | 1 | 185-formula integration suite (51/59 = 86.4%) |

### rtest Pass Rates
| Test file | Pass | Total | Rate |
|-----------|------|-------|------|
| rtest1 | 164 | 208 | 79% |
| rtest_boolean | 88 | 116 | 76% |
| rtest_equal | 112 | 208 | 54% |
| rtest_everysome | 45 | 84 | 54% |
| rtest_abs | 71 | 141 | 50% |
| rtest_dot | 36 | 72 | 50% |
| rtest_algebraic | 22 | 45 | 49% |

---

## S9: Advanced Integration Formulas + Risch Algebraic (Future)

### Goal
Extended integration table for radical/algebraic forms, reduction formulas,
and Risch algorithm for algebraic extensions.

### Integration table formulas (from reference table)

**Reduction formulas** (recursive, contain ∫ on RHS):
- #28-33: sin^n, cos^n, tan^n, cot^n, sec^n, csc^n general power reduction
- #39-40: u^n*sin(u), u^n*cos(u) by-parts reduction
- #43: u^n*exp(a*u) reduction
- #65-67: u^n*asin(u), u^n*acos(u), u^n*atan(u) reduction
- #111-113: u^n*sqrt(a+bu), u^n/sqrt(a+bu), 1/(u^n*sqrt(a+bu)) reduction

**Radical forms** (require algebraic substitutions):
- #68-76: sqrt(a²+u²) family — integral, u²*sqrt, sqrt/u, sqrt/u², 1/sqrt, u²/sqrt, 1/(u*sqrt), 1/(u²*sqrt), 1/(a²+u²)^(3/2)
- #77-84: sqrt(u²-a²) family — same structure
- #85-93: sqrt(a²-u²) family — same structure, includes (a²-u²)^(3/2)
- #94-97: sqrt(2au-u²) forms (completing the square to circular)

**Parametric rational forms**:
- #98-104: u/(a+bu), u²/(a+bu), 1/(u*(a+bu)), u/(a+bu)², etc.
- #105-110: u*sqrt(a+bu), u/sqrt(a+bu), u²/sqrt(a+bu), 1/(u*sqrt(a+bu)), sqrt(a+bu)/u, sqrt(a+bu)/u²

**Product trig forms** (require product-to-sum identities):
- #34-36: sin(au)*sin(bu), cos(au)*cos(bu), sin(au)*cos(bu)

**Conditional forms**:
- #108: 1/(u*sqrt(a+bu)) — branches on sign of a

### Integration Formula Test Results (86.4%)
185 formulas total → 59 tested (non-parametric, supported functions):
- **51 pass**: basic trig, hyp, inverse trig, exp/log, products, powers, substitutions
- **8 fail**: complex products (sec·csc, sin²cos²), 3-function products (x·eˣ·sin), exp·tanh
- **27 skipped**: unsupported functions (erf, gamma_incomplete, asec)
- **99 skipped**: parametric with free variables (a, b, n)

### Risch algebraic tasks
- [→v4] Algebraic number field extensions → V4.2
- [→v4] Radical Risch → V4.4
- [→v4] Trager's algorithm → V4.4

---

## References

1. Bronstein, *Symbolic Integration I — Transcendental Functions*, 2nd ed., Springer 2005
2. Geddes, Czapor, Labahn, *Algorithms for Computer Algebra*, Kluwer 1992
3. Gruntz, *On Computing Limits in a Symbolic Manipulation System*, ETH PhD 1996
4. Petkovšek, Wilf, Zeilberger, *A=B*, A K Peters 1996
5. Boettner, *Mixed Transcendental and Algebraic Extensions for the Risch-Norman Algorithm*, Tulane PhD 2010
6. FriCAS source: `intef.spad`, `intrf.spad`, `intalg.spad`
7. SymPy source: `gruntz.py`, `risch.py`
8. SymbolicIntegration.jl (Julia, 2025)
