# RC6 — Equation Solving + Linear Algebra (New)

**Goal:** Implement equation solving (polynomial, transcendental) and
matrix operations (determinant, inverse, eigenvalues).

---

## Sprint 6.1 — Polynomial Equation Solving

**Duration:** 3 weeks

### Algorithms

**Linear:** Direct solution `ax + b = 0 → x = -b/a`

**Quadratic:** Quadratic formula with discriminant analysis.

**Cubic/Quartic:** Cardano's formula, Ferrari's method.
Only for exact solutions; numeric otherwise.

**General polynomial:** 
- Try rational roots (rational root theorem)
- Factor the polynomial (RC4 factoring)
- Each irreducible factor of degree ≤ 4: closed-form solution
- Degree > 4: return implicit solution or numeric roots

**Systems of equations (via Gröbner basis):**
```
1. Compute Gröbner basis with lex ordering
2. Last polynomial is univariate → solve it
3. Back-substitute to find other variables
```

### Tasks

- [ ] `solve(expr, x)` — single equation
- [ ] `solve([eq1, eq2], [x, y])` — system
- [ ] Linear system solving (Gaussian elimination)
- [ ] Quadratic formula with radical simplification
- [ ] Cubic/quartic (Cardano, Ferrari)
- [ ] Rational root theorem
- [ ] `solve` using factorization: `f(x)=0 ↔ factor_i(x)=0`
- [ ] `algsys` — algebraic system solving via Gröbner
- [ ] `linsolve` — dedicated linear system solver
- [ ] Solution multiplicity tracking

---

## Sprint 6.2 — Matrix Operations

**Duration:** 3 weeks

### Algorithms

**Determinant:** 
- Bareiss algorithm (fraction-free Gaussian elimination)
- Avoids rational arithmetic, works over Z
- O(n³) with exact integer arithmetic

**Inverse:** 
- Adjugate method: `A⁻¹ = adj(A) / det(A)`
- Or row reduction with augmented matrix

**Eigenvalues:**
- Compute characteristic polynomial `det(A - λI)`
- Factor and solve (uses RC4 factoring + RC6.1 solving)

**Rank:** Row echelon form, count pivots.

### Tasks

- [ ] Matrix type (list of lists internally)
- [ ] `matrix([row1], [row2], ...)` — construction
- [ ] `determinant(M)` — Bareiss algorithm
- [ ] `invert(M)` — matrix inverse
- [ ] `transpose(M)`
- [ ] `rank(M)` — via row echelon
- [ ] `eigenvalues(M)` — characteristic polynomial
- [ ] `eigenvectors(M)`
- [ ] Matrix arithmetic: `M1 + M2`, `M1 . M2`
- [ ] `ident(n)`, `zeromatrix(m,n)`, `diagmatrix(n,x)`
- [ ] `row(M,i)`, `col(M,j)`, `submatrix`
- [ ] `charpoly(M, x)` — characteristic polynomial

---

## Sprint 6.3 — File Loading + .mac Compatibility

**Duration:** 3 weeks

### Tasks

- [ ] `load(filename)` — find and execute .mac files
- [ ] File search paths: `file_search_maxima`, `file_search_tests`
- [ ] `batch(filename)` — batch execution
- [ ] `file_search(name, paths)` — locate file
- [ ] Autoload mechanism for `share/` packages
- [ ] `save`/`restore` — session persistence

---

## Sprint 6.4 — Display + TeX

**Duration:** 2 weeks

### Tasks

- [ ] 2D ASCII display (fractions, exponents, matrices)
- [ ] `tex(expr)` — LaTeX output
- [ ] `grind(expr)` — re-parseable 1D output
- [ ] Line width handling and wrapping
