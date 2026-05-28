# RC4 — Polynomial System (Revised)

**Goal:** Implement the Canonical Rational Expression (CRE) system —
the backbone of all symbolic computation in Maxima. This replaces the
ad-hoc simplifier with proper polynomial arithmetic.

**Mathematical foundation:** Sparse recursive multivariate polynomials
over Z (or Q), with GCD-based simplification, and Gröbner basis support
for ideal-theoretic operations.

---

## Sprint 4.1 — Sparse Polynomial Representation

**Duration:** 3 weeks

### Data Structure

Maxima uses a recursive sparse representation. A polynomial in `x`
with coefficients that are polynomials in `y,z,...`:

```rust
/// Sparse polynomial: list of (exponent, coefficient) pairs,
/// sorted by descending exponent.
/// Coefficient is either an integer or another Polynomial.
pub struct Poly {
    var: SymbolId,
    terms: Vec<(u32, Coeff)>,
}

pub enum Coeff {
    Int(i64),
    BigInt(Box<BigInt>),
    Rational(i64, i64),
    Poly(Box<Poly>),
}
```

This mirrors Maxima's internal format where `(x 3 1 2 5 0 3)` means
`x^3 + 2*x^2 + 3` (alternating exponent-coefficient pairs).

### Variable Ordering

CRE requires a total order on variables. The order determines which
variable is "outermost" in the recursive representation.

```
Default: alphabetical (a > b > c > x > y > z)
User override: ratvars(x, y, z) sets x > y > z
```

The ordering affects canonical form: `x*y + 1` in order `x > y` is
`Poly{x, [(1, Poly{y, [(1,1)]}), (0, 1)]}` but in order `y > x` is
`Poly{y, [(1, Poly{x, [(1,1)]}), (0, 1)]}`.

### Tasks

- [ ] `Poly` struct with sparse term list
- [ ] `Coeff` enum (Int, BigInt, Rational, nested Poly)
- [ ] Variable ordering table (global, user-configurable)
- [ ] `Poly::from_expr(expr, vars)` — convert Expr to Poly
- [ ] `Poly::to_expr()` — convert Poly back to Expr
- [ ] `Poly::degree()`, `Poly::leading_coeff()`
- [ ] `Poly::is_zero()`, `Poly::is_constant()`
- [ ] Display for Poly (debugging)

### Tests

```
Poly::from_expr(x^2+2*x+1) → terms: [(2,1),(1,2),(0,1)]
Poly::from_expr(x*y+1)     → terms: [(1, Poly{y,[(1,1)]}), (0,1)]
roundtrip: from_expr → to_expr → from_expr = identity
```

---

## Sprint 4.2 — Polynomial Arithmetic

**Duration:** 2 weeks

### Algorithms

**Addition:** Merge sorted term lists, combining like exponents.
O(n+m) where n,m are term counts.

**Multiplication:** Convolution of term lists with recursive
coefficient multiplication. O(n*m) terms, then collect like exponents.

**Division:** Polynomial long division. Returns (quotient, remainder).
Only exact division when remainder is zero.

**Pseudo-division:** For GCD computation. Divides without requiring
exact coefficient division by multiplying by leading coefficient.

```rust
impl Poly {
    fn add(&self, other: &Poly) -> Poly;
    fn sub(&self, other: &Poly) -> Poly;
    fn mul(&self, other: &Poly) -> Poly;
    fn divmod(&self, other: &Poly) -> (Poly, Poly);
    fn pseudo_divmod(&self, other: &Poly) -> (Poly, Poly, Coeff);
    fn neg(&self) -> Poly;
    fn scale(&self, c: &Coeff) -> Poly;
}
```

### Tasks

- [ ] Addition with term merging
- [ ] Subtraction
- [ ] Multiplication with convolution
- [ ] Polynomial long division
- [ ] Pseudo-division for GCD
- [ ] Scalar multiplication and division
- [ ] Power (repeated squaring)
- [ ] Content and primitive part

### Tests

```
(x+1)*(x-1) = x^2-1
(x^3-1) / (x-1) = x^2+x+1, remainder 0
(x^3+1) / (x+1) = x^2-x+1, remainder 0
content(6*x^2+4*x+2) = 2, primpart = 3*x^2+2*x+1
```

---

## Sprint 4.3 — Polynomial GCD

**Duration:** 3 weeks

### Algorithms

**Euclidean GCD:** Simple but suffers from coefficient explosion
in multivariate case.

**Subresultant GCD:** Controls coefficient growth via subresultant
chain. Standard choice for multivariate polynomials over Z.

**Modular GCD (optional, for performance):** Compute GCD mod several
primes, then reconstruct via Chinese Remainder Theorem + Hensel lifting.
This is what Maxima uses for large polynomials.

```
Algorithm: Subresultant GCD
Input: polynomials f, g in Z[x]
Output: gcd(f, g)

1. If deg(g) > deg(f), swap f, g
2. While g ≠ 0:
   a. Compute pseudo-remainder r = prem(f, g)
   b. Reduce r by subresultant factor
   c. f ← g, g ← r
3. Return primitive_part(f)
```

For multivariate: recursive — GCD of polynomials in x with
coefficients in Z[y,z,...] reduces to GCD of the coefficients.

### Tasks

- [ ] Euclidean GCD for univariate
- [ ] Subresultant PRS (Polynomial Remainder Sequence)
- [ ] Multivariate GCD via recursive reduction
- [ ] `gcd(poly1, poly2)` public interface
- [ ] `lcm(poly1, poly2)` via `lcm = a*b/gcd(a,b)`
- [ ] Cofactors: `gcd(a,b)` returning `(g, a/g, b/g)`
- [ ] Integration: `ratsimp` uses poly GCD for cancellation

### Tests

```
gcd(x^2-1, x^2+2*x+1) = x+1
gcd(x^3-1, x^6-1)     = x^3-1
gcd(x^2*y-y, x*y^2-x) = x*y-1 (or similar)
ratsimp((x^2-1)/(x-1)) = x+1
```

---

## Sprint 4.4 — Factoring over Z

**Duration:** 4 weeks

### Algorithms

**Square-free factorization (Yun's algorithm):**
```
Input: polynomial f
Output: f = f1 * f2^2 * f3^3 * ...

1. g ← gcd(f, f')
2. f* ← f/g
3. Iterate: extract gcd(f*, g), reduce g
```

**Berlekamp's algorithm (mod p):**
Factor polynomial mod a prime p. Construct the Berlekamp matrix,
find its null space, and split the polynomial accordingly.

**Hensel lifting:**
Lift factorization mod p to factorization mod p^k, then recover
factors over Z using the Landau-Mignotte bound.

**Kronecker's method (small degree fallback):**
For low-degree polynomials, try all divisors of the constant term.

```
Full pipeline:
1. Square-free decomposition (Yun)
2. For each square-free factor:
   a. Choose a prime p
   b. Factor mod p (Berlekamp)
   c. Lift to Z (Hensel)
   d. Recombine trial factors
```

### Tasks

- [ ] Square-free factorization (Yun)
- [ ] Factoring mod p (Berlekamp)
- [ ] Hensel lifting
- [ ] Factor recombination
- [ ] `factor(expr)` public interface
- [ ] `sqfr(expr)` — square-free part
- [ ] Bivariate factoring (Hensel in 2 variables)
- [ ] `resultant(p, q, var)`
- [ ] `discriminant(poly, var)`

### Tests

```
factor(x^2-1)       → (x-1)*(x+1)
factor(x^4-1)       → (x-1)*(x+1)*(x^2+1)
factor(x^6-1)       → (x-1)*(x+1)*(x^2-x+1)*(x^2+x+1)
factor(2*x^2+4*x+2) → 2*(x+1)^2
sqfr(x^3-3*x^2+3*x-1) → (x-1)^3
```

---

## Sprint 4.5 — Partial Fractions + Rational Simplification

**Duration:** 2 weeks

### Algorithm

Partial fraction decomposition over Z[x]:
```
Input: p(x)/q(x) where deg(p) < deg(q)
1. Factor q(x) = q1^e1 * q2^e2 * ...
2. For each factor qi^ei:
   Solve for coefficients Aij in
   p/q = Σ Aij / qi^j  (j = 1..ei)
3. Use Hermite reduction for repeated factors
```

### Tasks

- [ ] `partfrac(expr, var)` — partial fraction decomposition
- [ ] `combine(expr)` — combine fractions
- [ ] `ratsimp` using polynomial GCD (proper version)
- [ ] `xthru(expr)` — multiply through by denominator
- [ ] `ratvars(v1, v2, ...)` — set variable ordering

---

## Sprint 4.6 — Gröbner Bases (Foundation)

**Duration:** 3 weeks

### Algorithm

**Buchberger's algorithm:**
```
Input: set of polynomials F = {f1, ..., fm} in k[x1,...,xn]
Output: Gröbner basis G for the ideal ⟨F⟩

1. G ← F
2. For each pair (fi, fj) in G:
   a. Compute S-polynomial S(fi, fj)
   b. Reduce S mod G → r
   c. If r ≠ 0, add r to G
3. Repeat until no new elements added
4. Reduce G (remove redundant elements)
```

**Monomial orderings:**
- Lexicographic (lex): x₁ > x₂ > ... (for elimination)
- Graded reverse lex (grevlex): total degree first (faster computation)
- Degree lex (grlex): for compatibility

**Applications:**
- `solve` for polynomial systems
- `eliminate` variables from system
- `ideal_membership` testing
- `algsys` (algebraic system solving)

### Tasks

- [ ] Monomial type with ordering support (lex, grevlex, grlex)
- [ ] S-polynomial computation
- [ ] Polynomial reduction (division by a set)
- [ ] Buchberger's algorithm (basic version)
- [ ] Reduced Gröbner basis
- [ ] `groebner(polys, vars)` interface
- [ ] `eliminate(eqns, vars)` using lex ordering
- [ ] Integration with `solve` for polynomial systems

### Tests

```
groebner([x^2+y^2-1, x-y], [x,y])
  → [y^2-1/2, x-y] (or equivalent)

eliminate([x+y=1, x*y=1], [y])
  → [x^2-x+1=0]
```

### Performance Notes

Gröbner basis computation is EXPSPACE-complete in the worst case.
For practical use:
- Implement degree bounds and timeout
- Use grevlex for computation, convert to lex for elimination
- Consider Faugère's F4/F5 algorithms for performance (future)
- Cache computed bases
