# Algorithm Roadmap

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  RC3: Assumptions              RC4: Polynomial System               │
│  ┌──────────────┐             ┌─────────────────────────┐          │
│  │ Fact Database │             │ Sparse Poly Repr (4.1)  │          │
│  │ Sign Inference│             │         ↓               │          │
│  │ Contexts     │             │ Poly Arithmetic (4.2)   │          │
│  └──────┬───────┘             │         ↓               │          │
│         │                     │ Polynomial GCD (4.3)    │          │
│         │                     │    ↓         ↓          │          │
│         │                     │ Factor(4.4) Partfrac(4.5)│         │
│         │                     │    ↓                    │          │
│         │                     │ Gröbner Basis (4.6)     │          │
│         │                     └────────────┬────────────┘          │
│         │                                  │                        │
│         ▼                                  ▼                        │
│  ┌──────────────────────────────────────────────────────┐          │
│  │              RC5: Integration + Limits                │          │
│  │  ┌──────────────────┐  ┌────────────────────┐       │          │
│  │  │ Risch Algorithm  │  │ Gruntz (Limits)    │       │          │
│  │  │  - Hermite       │  │  - MRV set         │       │          │
│  │  │  - Rothstein     │  │  - Growth compare  │       │          │
│  │  │  - Transcendental│  │  - Series expand   │       │          │
│  │  └──────────────────┘  └────────────────────┘       │          │
│  └──────────────────────────────────────────────────────┘          │
│                            │                                        │
│                            ▼                                        │
│  ┌──────────────────────────────────────────────────────┐          │
│  │              RC6: Solving + Linear Algebra            │          │
│  │  ┌──────────────────┐  ┌────────────────────┐       │          │
│  │  │ Solve (poly)     │  │ Matrix Operations  │       │          │
│  │  │  - Factor+roots  │  │  - Bareiss det     │       │          │
│  │  │  - Gröbner sys   │  │  - Inverse         │       │          │
│  │  │  - Cardano etc   │  │  - Eigenvalues     │       │          │
│  │  └──────────────────┘  └────────────────────┘       │          │
│  └──────────────────────────────────────────────────────┘          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Algorithms by Complexity

### Well-Understood (implement directly)

| Algorithm | Sprint | Complexity | Notes |
|-----------|--------|------------|-------|
| Polynomial addition/multiplication | 4.2 | O(n*m) | Straightforward |
| Euclidean GCD | 4.3 | O(n²) coeff growth | Simple but slow |
| Subresultant GCD | 4.3 | O(n²) no growth | Standard for CAS |
| Square-free factorization (Yun) | 4.4 | O(n²) | GCD-based |
| Hermite reduction | 5.1 | O(n²) | Extended Euclidean |
| Taylor expansion | 5.4 | O(n) diffs | Automatic diff |
| Gaussian elimination | 6.1 | O(n³) | Standard |
| Bareiss determinant | 6.2 | O(n³) | Fraction-free |

### Moderate Complexity (careful implementation)

| Algorithm | Sprint | Complexity | Notes |
|-----------|--------|------------|-------|
| Berlekamp factoring mod p | 4.4 | O(n³) | Matrix null space |
| Hensel lifting | 4.4 | O(n² log p^k) | p-adic lifting |
| Rothstein-Trager | 5.1 | O(n²) + factoring | Resultant-based |
| Risch-Norman heuristic | 5.2 | Heuristic | May fail |
| Gruntz limits | 5.3 | Recursive | MRV computation |
| Cardano/Ferrari | 6.1 | Closed-form | Radical expressions |

### High Complexity (defer or simplify)

| Algorithm | Sprint | Complexity | Notes |
|-----------|--------|------------|-------|
| Buchberger (Gröbner) | 4.6 | EXPSPACE worst | Use degree bounds |
| Full Risch algorithm | 5.2 | Decidable but complex | Start with heuristic |
| Multivariate factoring | 4.4+ | Hard | Bivariate first |
| Algebraic extensions | Future | Very complex | tellrat system |

## Rust-Specific Design Decisions

### Memory Management

```rust
// Arena allocator for polynomial terms (avoid per-term allocation)
struct PolyArena {
    terms: Vec<(u32, Coeff)>,
    polys: Vec<PolyRef>,  // indices into terms
}
```

### Parallelism Opportunities

- Independent GCD computations (cofactors)
- Modular GCD: compute mod different primes in parallel
- Matrix operations: row operations in parallel
- Gröbner: S-polynomial computation parallelizable

### Trait-Based Dispatch

```rust
trait Ring: Clone + PartialEq {
    fn zero() -> Self;
    fn one() -> Self;
    fn add(&self, other: &Self) -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn neg(&self) -> Self;
}

trait EuclideanDomain: Ring {
    fn divmod(&self, other: &Self) -> (Self, Self);
    fn gcd(&self, other: &Self) -> Self;
}

// Poly<R> is a polynomial with coefficients in R
struct Poly<R: Ring> {
    var: SymbolId,
    terms: Vec<(u32, R)>,
}
```

This allows the same GCD algorithm to work over Z, Q, Z[y], etc.

## Timeline (Revised)

| Phase | Duration | Key Deliverable |
|-------|----------|-----------------|
| RC3: Assumptions | 6 weeks | assume/is/asksign working |
| RC4: Polynomials | 14 weeks | GCD, factor, Gröbner |
| RC5: Integration | 10 weeks | ∫ rational + basic transcendental |
| RC6: Solving | 8 weeks | solve polynomial systems |
| **Total** | **38 weeks** | |

## References

- Cohen, "Computer Algebra and Symbolic Computation: Mathematical Methods" (2003)
- Geddes, Czapor, Labahn, "Algorithms for Computer Algebra" (1992)
- von zur Gathen, Gerhard, "Modern Computer Algebra" (2013)
- Bronstein, "Symbolic Integration I: Transcendental Functions" (2005)
- Gruntz, "On Computing Limits in a Symbolic Manipulation System" (1996)
