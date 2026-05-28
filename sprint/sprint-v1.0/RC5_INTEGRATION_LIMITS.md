# RC5 — Symbolic Integration + Limits (New)

**Goal:** Implement symbolic integration (Risch algorithm) and
limit computation (Gruntz algorithm). These are the crown jewels
of a CAS and depend heavily on the polynomial system from RC4.

**Prerequisite:** RC4 (polynomial GCD, factoring) must be solid.

---

## Sprint 5.1 — Risch Algorithm Foundation

**Duration:** 4 weeks

### Algorithm Overview

The Risch algorithm decides whether an elementary antiderivative exists
and computes it if so. It decomposes the problem by the tower of
transcendental extensions.

```
Integration pipeline:
1. Express integrand in terms of a tower: Q(x, θ₁, θ₂, ...)
   where each θᵢ is log(f) or exp(f) for some f in the previous field
2. For each extension, apply the appropriate integration rule:
   - Rational functions: Hermite reduction + Rothstein-Trager
   - Logarithmic extensions: Risch-Norman
   - Exponential extensions: Risch-Norman
   - Algebraic extensions: Trager's algorithm
```

### Hermite Reduction (Rational Integration)

```
Input: p(x)/q(x) where gcd(p,q)=1
1. Square-free decomposition of q: q = q₁ * q₂² * q₃³ * ...
2. For each qᵢ^i with i > 1:
   Extended Euclidean: s*qᵢ + t*qᵢ' = p mod qᵢ^i
   Gives: ∫p/qᵢ^i = -t/((i-1)*qᵢ^(i-1)) + ∫(s+(i-1)t'/((i-1)qᵢ^(i-1)))
3. Remaining: ∫p/q₁ handled by Rothstein-Trager (logarithmic part)
```

### Rothstein-Trager Algorithm

```
Input: p(x)/q(x) where q is square-free
Output: Σ cᵢ * log(uᵢ(x))

1. Compute resultant R(t) = res_x(p - t*q', q)
2. Factor R(t) — roots cᵢ are the log coefficients
3. For each root cᵢ: uᵢ = gcd(p - cᵢ*q', q)
```

### Tasks

- [ ] Rational function integration (polynomial part)
- [ ] Hermite reduction for proper rational functions
- [ ] Rothstein-Trager for logarithmic part
- [ ] `integrate(f, x)` for rational functions
- [ ] Handle table-lookup for standard forms:
  - `∫x^n dx = x^(n+1)/(n+1)` for n ≠ -1
  - `∫1/x dx = log(x)`
  - `∫exp(x) dx = exp(x)`
  - `∫sin(x) dx = -cos(x)`, etc.

---

## Sprint 5.2 — Transcendental Extensions

**Duration:** 4 weeks

### Algorithm

For integrating expressions with `log` and `exp`:

**Logarithmic case:** ∫f(x, log(g(x))) dx
- Differentiate the candidate antiderivative
- Solve for unknown coefficients using polynomial identity

**Exponential case:** ∫f(x, exp(g(x))) dx
- If g(x) = a*x + b (simple exponential), use substitution
- General: Risch differential equation

**Risch Differential Equation:**
```
y' + f*y = g  in the differential field K(θ)
where θ = log(u) or θ = exp(u)

Solution involves:
1. Bound the degree of y
2. Solve the resulting linear system
3. Check integrability conditions
```

### Tasks

- [ ] Integration of `exp(a*x+b) * polynomial(x)`
- [ ] Integration of `log(x) * polynomial(x)` (integration by parts)
- [ ] Risch-Norman heuristic for mixed transcendental
- [ ] `integrate(f, x, a, b)` — definite integration via FTC
- [ ] Noun form `'integrate(f,x)` when no antiderivative found

---

## Sprint 5.3 — Limit Computation (Gruntz Algorithm)

**Duration:** 3 weeks

### Algorithm

Gruntz's algorithm computes limits by comparing growth rates
of subexpressions using the MRV (Most Rapidly Varying) set.

```
Algorithm: lim(f(x), x → ∞)

1. Compute MRV set: subexpressions with maximal growth rate
2. Choose ω from MRV set
3. Rewrite f in terms of ω: f = c₀ + c₁*ω + c₂*ω² + ...
4. Recursively compute limit of leading coefficient
5. Determine sign of exponent to get limit
```

**Growth comparison:** For expressions e₁, e₂:
```
e₁ ≻ e₂  if  lim log(|e₁|) / log(|e₂|) → ∞
e₁ ≍ e₂  if  lim log(|e₁|) / log(|e₂|) → c ≠ 0
e₁ ≺ e₂  if  lim log(|e₁|) / log(|e₂|) → 0
```

### Tasks

- [ ] MRV set computation
- [ ] Growth rate comparison
- [ ] Series expansion at infinity
- [ ] `limit(f, x, a)` — limit at a point
- [ ] `limit(f, x, inf)` — limit at infinity
- [ ] `limit(f, x, a, plus)` / `limit(f, x, a, minus)` — one-sided
- [ ] L'Hôpital's rule as fallback
- [ ] Special limits: `sin(x)/x → 1`, etc.

---

## Sprint 5.4 — Taylor/Laurent Series

**Duration:** 2 weeks

### Algorithm

Taylor expansion using automatic differentiation:
```
taylor(f, x, a, n) = Σ f⁽ᵏ⁾(a)/k! * (x-a)^k  for k=0..n
```

For functions with poles, use Laurent series with negative powers.

### Tasks

- [ ] `taylor(f, x, a, n)` — Taylor expansion
- [ ] `powerseries(f, x, a)` — infinite series
- [ ] Truncated power series arithmetic
- [ ] `residue(f, x, a)` — residue at a pole
