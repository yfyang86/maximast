# Maxima Rust Kernel v5.0 — Standard Library Expansion

## Goal

Bring the Rust kernel to feature parity with common Maxima usage patterns.
Focus on the most-used functions that real scripts depend on, ordered by
impact × implementation cost.

## Current State (post-V4/V5 merge)

- 822 tests, 16 walkthroughs
- Core: arithmetic, algebra, calculus, solving, summation, matrices, assumptions
- Infrastructure: CLI, REPL with tab completion, package system, plugin API

## Sprint Plan

| Sprint | Content | Size | Priority |
|--------|---------|------|----------|
| **S1** | Sets and set algebra | Small | High |
| **S2** | String functions | Small | High |
| **S3** | Number theory | Small | High |
| **S4** | Polynomial analysis | Medium | High |
| **S5** | Log/trig simplification | Medium | Medium |
| **S6** | Expression manipulation | Small | Medium |
| **S7** | Laplace transforms | Large | Medium |
| **S8** | ODE solver (ode2) | Large | Lower |

---

## S1 — Sets and Set Algebra (Small, ~2 hours)

Maxima has a proper set type `{a, b, c}`. Our kernel stores sets as
sorted-unique lists. Implement set operations on this representation.

### Tasks

- [ ] `union(A, B)` — merge two sets (sorted unique merge)
- [ ] `intersection(A, B)` — elements in both
- [ ] `setdifference(A, B)` — elements in A not in B
- [ ] `symdifference(A, B)` — symmetric difference
- [ ] `subset(A, predicate)` — filter elements by predicate
- [ ] `subsetp(A, B)` — is A ⊆ B?
- [ ] `elementp(x, S)` — is x ∈ S?
- [ ] `cardinality(S)` — number of elements
- [ ] `powerset(S)` — all subsets of S
- [ ] `disjointp(A, B)` — are A and B disjoint?

### Verify

```
union({1,2,3}, {3,4,5});           → {1,2,3,4,5}
intersection({1,2,3}, {2,3,4});    → {2,3}
setdifference({1,2,3}, {2});       → {1,3}
cardinality({a,b,c});              → 3
powerset({1,2});                   → {{},{1},{2},{1,2}}
```

### Notes

`setify` and `listify` already exist. Sets are internally lists with
`MList` operator. We may add a `MSet` operator later but for now
sorted-unique lists suffice.

---

## S2 — String Functions (Small, ~2 hours)

Only `sconcat`, `concat`, and `string` exist. Add the standard string API.

### Tasks

- [ ] `slength(s)` — string length
- [ ] `charat(s, n)` — character at position (1-indexed)
- [ ] `substring(s, start)` and `substring(s, start, end)` — extract substring
- [ ] `ssearch(pattern, s)` — find substring position (false if not found)
- [ ] `ssubst(new, old, s)` — replace substring
- [ ] `strim(s)` — trim whitespace
- [ ] `split(s)` and `split(s, delim)` — split into list of strings
- [ ] `supcase(s)` / `sdowncase(s)` — case conversion
- [ ] `sequal(s1, s2)` — string equality
- [ ] `parse_string(s)` — parse string as Maxima expression

### Verify

```
slength("hello");                → 5
charat("hello", 2);             → "e"
substring("hello", 2, 4);       → "ell"
ssearch("ll", "hello");         → 3
ssubst("world", "hello", "hello world"); → "world world"
split("a,b,c", ",");            → ["a","b","c"]
parse_string("x^2+1");          → x^2+1
```

---

## S3 — Number Theory (Small, ~2 hours)

`primep`, `gcd`, `mod`, `binomial` exist. Add factorization and common
number-theoretic functions.

### Tasks

- [ ] `ifactors(n)` — integer factorization: [[p1,e1],[p2,e2],...]
- [ ] `totient(n)` — Euler's totient φ(n)
- [ ] `divisors(n)` — list of all divisors
- [ ] `next_prime(n)` — smallest prime > n
- [ ] `prev_prime(n)` — largest prime < n
- [ ] `power_mod(base, exp, modulus)` — modular exponentiation
- [ ] `inv_mod(a, n)` — modular inverse
- [ ] `jacobi(a, n)` — Jacobi symbol
- [ ] `chinese([r1,r2,...], [m1,m2,...])` — Chinese Remainder Theorem
- [ ] `fibonacci(n)` — nth Fibonacci number (fast doubling)

### Verify

```
ifactors(360);                   → [[2,3],[3,2],[5,1]]
totient(12);                     → 4
divisors(12);                    → [1,2,3,4,6,12]
next_prime(100);                 → 101
power_mod(2, 100, 1000000007);   → 976371285
chinese([2,3,2], [3,5,7]);      → 23
fibonacci(50);                   → 12586269025
```

---

## S4 — Polynomial Analysis (Medium, ~3 hours)

We have `factor`, `gcd`, `coeff`, `hipow`. Add missing polynomial tools.

### Tasks

- [ ] `lopow(expr, var)` — lowest power of var in expression
- [ ] `content(poly, var)` — GCD of all coefficients
- [ ] `primpart(poly, var)` — poly / content(poly, var)
- [ ] `resultant(p, q, var)` — Sylvester resultant (eliminant)
- [ ] `discriminant(p, var)` — discriminant of polynomial
- [ ] `sqfr(poly)` — square-free factorization (already partial, verify)
- [ ] `nroots(poly, lo, hi)` — number of real roots in interval (Sturm)
- [ ] `realroots(poly)` — isolate all real roots

### Verify

```
resultant(x^2+a*x+b, x^2+c*x+d, x);  → (b-d)^2-(a-c)*(b*c-a*d)
discriminant(a*x^2+b*x+c, x);          → b^2-4*a*c
content(6*x^2+4*x+2, x);              → 2
lopow(x^3+x, x);                       → 1
```

---

## S5 — Log/Trig Simplification (Medium, ~3 hours)

### Tasks

- [ ] `logcontract(expr)` — combine logs: `log(a)+log(b)` → `log(a*b)`
- [ ] `logexpand(expr)` — expand logs: `log(a*b)` → `log(a)+log(b)`
- [ ] `log_simp` flag — auto-simplify log expressions
- [ ] `halfangles(expr)` — convert sin(x) → in terms of tan(x/2)
- [ ] `trigrat(expr)` — rational trig form
- [ ] Improve `trigreduce` coverage for products: `sin(a)*cos(b)` → sum

### Verify

```
logcontract(log(x)+log(y));      → log(x*y)
logexpand(log(x*y));             → log(x)+log(y)
logcontract(2*log(x));           → log(x^2)
trigreduce(sin(x)*cos(x));       → sin(2*x)/2
```

---

## S6 — Expression Manipulation (Small, ~1.5 hours)

### Tasks

- [ ] `multthru(expr)` — distribute multiplication over addition
- [ ] `xthru(expr)` — put over common denominator without expanding
- [ ] `collectterms(expr, var)` — collect terms by powers of var
- [ ] `at(expr, [x=a, y=b])` — evaluate at multiple substitutions
- [ ] `lfreeof(list, expr)` — true if expr is free of all vars in list
- [ ] `nterms(expr)` — number of terms (top-level addends)

### Verify

```
multthru(a*(b+c));               → a*b+a*c
multthru((a+b)/c);               → a/c+b/c
xthru(a/b+c/d);                  → (a*d+b*c)/(b*d)
at(x^2+y, [x=3, y=1]);          → 10
collectterms(a*x+b*x+c, x);     → (a+b)*x+c
```

---

## S7 — Laplace Transforms (Large, ~5 hours)

A table-driven Laplace transform engine covering common patterns.

### Tasks

- [ ] `laplace(expr, t, s)` — forward Laplace transform
- [ ] `ilt(expr, s, t)` — inverse Laplace transform
- [ ] Table entries for: polynomials, exp, sin, cos, sinh, cosh, step functions
- [ ] Linearity: `L{af+bg} = aL{f}+bL{g}`
- [ ] Shift theorems: `L{exp(at)f(t)} = F(s-a)`
- [ ] Derivative rule: `L{f'(t)} = sF(s)-f(0)`
- [ ] Convolution: `L{f*g}(s) = F(s)G(s)`

### Verify

```
laplace(t^n, t, s);              → n!/s^(n+1)
laplace(exp(a*t), t, s);        → 1/(s-a)
laplace(sin(w*t), t, s);        → w/(s^2+w^2)
ilt(1/(s-a), s, t);             → exp(a*t)
ilt(s/(s^2+w^2), s, t);         → cos(w*t)
```

---

## S8 — ODE Solver (Large, ~8 hours)

`ode2` for first and second order ODEs.

### Tasks

- [ ] `ode2(eqn, y, x)` — solve ODE
- [ ] First-order methods: separable, linear, exact, homogeneous, Bernoulli
- [ ] Second-order: constant-coefficient (homogeneous + variation of parameters)
- [ ] `ic1(sol, x=a, y=b)` — apply initial condition to first-order
- [ ] `ic2(sol, x=a, y=b, 'diff(y,x)=c)` — apply IC to second-order
- [ ] `bc2(sol, x=a, y=b, x=c, y=d)` — boundary conditions

### Verify

```
ode2('diff(y,x)+y=0, y, x);                → y=%c*exp(-x)
ode2('diff(y,x,2)+y=0, y, x);              → y=%k1*sin(x)+%k2*cos(x)
ode2('diff(y,x)=x*y, y, x);               → y=%c*exp(x^2/2)
ic1(ode2('diff(y,x)+y=0,y,x), x=0, y=1);  → y=exp(-x)
```

---

## Implementation Order

**Phase 1 (Quick wins)**: S1 → S2 → S3 → S6 (7-8 hours)
Focus: fill the most obvious gaps that real scripts hit.

**Phase 2 (Core math)**: S4 → S5 (6 hours)
Focus: algebraic infrastructure for more advanced work.

**Phase 3 (Advanced)**: S7 → S8 (13 hours)
Focus: transforms and ODEs — these are large but high-value features.

## Future Work (beyond V5.0)

| Item | Description |
|------|-------------|
| Euler substitution | Correct implementation of ∫ R(x,√(ax²+bx+c)) |
| Matrix algebra | Element-wise +/-/*, Kronecker product, rank |
| Pattern matching | matchdeclare/defrule/tellsimp |
| Plotting | plot2d/plot3d via gnuplot |
| Floating-point | bfloat (arbitrary precision float) |
| Complex analysis | residue, contour integration |
| Tensor algebra | itensor/ctensor packages |
| Dynamic plugins | .so/.dylib loading via dlopen |
