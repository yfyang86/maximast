# Maxima Rust Kernel v13 — Gap-closing program (4 bundles)

Driven by `research/survey/FUTURE_SPRINTS.md`. Four bundles, in order. Same
discipline: **compute → verify → return; correct-or-noun, never wrong.**

## Bundle 1 — Trust & polish (Tier 0 correctness) 🚧

| # | Fix | Status |
|---|-----|--------|
| 0d | `(-1)^(2n)` printed as `-1^(2n)` — parenthesize negative/rational bases | ✅ |
| 0e | expand-before-integrate; `∫x^n = x^(n+1)/(n+1)` (n≠−1) | 📋 |
| 0g | numeric `fib`/`lucas`; exact `rank` (not f64); square-free Sturm | 📋 |
| 0a | parametric/symbolic `linsolve` (was `[x=0,y=0]`) | ✅ |
| 0b | infinite sums: convergent geometric exact, rest noun (was substituting `inf`) | ✅ |
| 0c | definite-integral `inf`-leak gating (→ noun) | ✅ |
| 0f | iterated-squaring timeout — **re-scoped**: real cost in `expand` (4097-term poly²); needs fast poly-expand / hash-consing (→ infra 1e), not the simplify flag (ineffective + flag unreliable) | ⏭️ |
| 0h | plugin name resolution ✅; parser `,numer` panic → deferred (Result-based parser refactor; ev-modifier is a feature) | ◑ |

## Bundle 2 — Solve & numbers 🚧

- **1a-i** ✅ radical solve via factor decomposition: each factor solved by
  degree — linear → rational, quadratic → `-b/(2a)±√((b²-4ac)/(4a²))` (clean √
  and complex %i), biquadratic quartic → quadratic-in-x². All-or-noun.
  `solve(x^2+1)=±%i`, `solve(x^4-5x^2+6)=±√2,±√3`, `solve(x^4-4x^2+1)=±√(2±√3)`,
  `solve(x^4-1)=±1,±%i`, `solve(x^3-1)=1,(-1±%i√3)/2`. (Used meval for radicand
  reduction — simplify alone doesn't reduce div(12,4)→3, a noted gap.)
- **1a-ii** ◑ Cardano pure-cube (`solve(x^3-2)=2^(1/3),2^(1/3)ω,2^(1/3)ω²`): depress
  to t³+pt+q, handle p=0 (t³=−q → k^(1/3)·ω^j). Casus irreducibilis (p≠0, 3 real
  roots) deferred → noun.
  TODO: general Cardano (p≠0), Ferrari (general quartic),
  `RootOf` object (architectural — sign-off first), `polysys_solve` cascade.


1a cubic/quartic radical solve + `RootOf` · 1b exact real-root isolation ·
1c arbitrary-precision bigfloat backend · 3a matrix decompositions · 3b general
eigen · 3c special-function numeric eval · 3d numeric solvers/quadrature/ODE.

## Bundle 3 — Summation completion 🚧

2a order-≥2 Zeilberger (proven certificate) · 2b harmonic/Karr–Schneider sums ·
3k generating functions / holonomic→GF.

- **2b** ✅ (harmonic): `sum(1/k^p,k,1,n)` → `harmonic(n)` / `harmonic(n,p)`;
  `harmonic(m[,p])` evaluates to the exact rational at integer m (so the closed
  form round-trips to the numeric sum). Infinite: p=1 → `inf` (divergent),
  p≥2 → `zeta(p)` (`%pi^2/6`, `%pi^4/90`, … for even p). Full Karr–Schneider
  ΠΣ (nested S-sums) still future.
- **3k** ✅ (generating functions): `sum(p(k)·xᵏ,k,0/1,inf)` → rational in x via
  the `(x·d/dx)` operator on `1/(1-x)`. Symbolic base (`x/(1-x)^2`) and numeric
  base with |base|<1 (`sum(k·(1/2)^k)=2`); every closed form numerically
  verified at a convergent point, divergent series → noun. (`genfunc.rs`.)
- **2a** order-≥2 Zeilberger proven certificate — `find_recurrence` already
  emits the recurrence (Franel/Apéry, exact + held-out-verified). Certificate
  engine foundation **landed**: the symbolic Gosper now handles binomials
  (reduced to factorials in the shift ratio), so binomial terms telescope and
  carry WZ certificates (`Σ binomial(k,m)=binomial(n+1,m+1)`;
  `gosper_certificate(binomial(k,2))=(k-2)/3`). **Remaining (→ Bundle 4):** the
  symbolic *parametrized* Gosper over Q(n) — the existing Gosper/Petkovsek
  linear algebra is over Q, and the Zeilberger combination `Σ c_j(n)·F(n+j,k)`
  needs both a similar-hypergeometric-sum front-end and Q(n) coefficients
  (confirmed: a rushed reuse of the numeric Gosper can't produce the symbolic
  certificate). A new Q(n)[k] polynomial layer is the right next step.

## Bundle 4 — Analysis 🚧

1d inverse Laplace (residues) · 2e contour/residue definite integrals ·
3e Fourier transforms · 3f Frobenius/Euler ODE · 3g `desolve`/ODE systems.

- **1d** ✅ inverse Laplace of a general rational F(s)=N/D via exact partial
  fractions over Q (`laplace.rs`): D factored into linear and irreducible-
  quadratic factors, PFD numerators by an exact ℚ linear solve, each term
  inverted by transform pairs — `A/(s−a)^j → A·t^(j−1)·e^(at)/(j−1)!` (any
  multiplicity) and `(Bs+C)/((s+p)²+ω²) → e^(−pt)[B cos ωt + ((C−Bp)/ω) sin ωt]`.
  `ilt(1/(s²+1))=sin t`, `ilt(1/(s²−1))=sinh t`, repeated poles, damped
  oscillations; verified by `laplace(ilt(F))=F` round-trip. Repeated complex
  poles (quadratic mult ≥2) → noun. (Also a foundation for 3g `desolve`.)
- **3f** ✅ (Euler–Cauchy) — `ode2` now solves variable-coefficient
  `A·x²y'' + B·x·y' + C·y = 0` via the indicial equation `A·m²+(B−A)m+C=0`:
  distinct real roots → `x^m₁,x^m₂`; repeated → `(k1+k2 ln x)x^m`; complex p±qi
  → `x^p(k1 cos(q ln x)+k2 sin(q ln x))`. Each solution is substituted back and
  required to vanish (correct-or-noun). Frobenius series fallback still future.
- **3g** ✅ (`desolve`) — linear constant-coefficient ODEs by the Laplace
  method: transform (`L{y'}=sY−y(0)`, `L{y''}=s²Y−s·y(0)−y'(0)`), solve the
  algebraic equation for Y(s), invert with `ilt`. Y splits by linearity so each
  `ilt` piece has rational coefficients. Initial values from `atvalue` (new),
  else symbolic `y(0)`/`at('diff(y,t),t=0)`. ODE systems still future.
- **2e** ✅ (residue definite integrals) — `∫_{−∞}^{∞} P/Q dx` via the
  upper-half-plane contour, realised as exact partial fractions over Q: each
  simple/repeated irreducible quadratic `(Bx+C)/((x−α)²+ω²)^m` integrates to
  `(Bα+C)·π·C(2m−2,m−1)/4^(m−1)/ω^(2m−1)`. `∫1/(x²+2x+5)=π/2`,
  `∫1/(x²+1)³=3π/8` (also fixed a wrong `∫1/(x²+1)²=π/4`→`π/2`). A real pole
  (linear factor) → noun (divergent). Shares the PFD engine with `ilt`.
  **2e extended** ✅: Fourier/Jordan integrals `∫_{−∞}^{∞} cos(ax)·P/Q` and
  `sin(ax)·P/Q` via `(π/ω)e^(−aω)[…]` per quadratic (`∫cos(x)/(x²+1)=πe^{−1}`,
  `∫x sin(x)/(x²+1)=πe^{−1}`); unit-circle `∫₀^{2π} c/(a+b·cos θ)=c·2π/√(a²−b²)`
  (and the sin analogue). **Remaining (→ 2c algebraic numbers):** Q
  irreducible-over-Q of degree ≥4 (`x⁴+1`) needs ℝ-factorisation with irrational
  coefficients; repeated/general unit-circle integrands.
- **3e** ✅ (Fourier transforms) — `fourier_transform(f,x,w)=∫f(x)e^(−iωx)dx`
  via the canonical pairs: Gaussian `exp(−ax²)→√(π/a)e^(−ω²/4a)`, two-sided
  exponential `exp(−a|x|)→2a/(a²+ω²)`, Lorentzian `1/(x²+a²)→(π/a)e^(−a|ω|)`,
  with linearity and constant factoring (cross-checks the 2e Fourier integrals
  at ω=1). General rational FT via 2e residues still future.

## Progress notes

- **Bundle 1a** ✅ (PR): 0d negative/rational power-base parens; 0e expand-before-
  integrate (polynomial-gated) + symbolic `∫x^n`; 0g numeric `fib`/`lucas`
  (`find_recurrence(fib(n))=[-1,-1,1]`). Bundle 1 cheap items done. Remaining/deferred: 0f (→ infra 1e fast poly-expand/
hash-consing), parser robustness (Result-based parser), 0i (gruntz limit bugs),
0j (`1/(1/2)` simplify). Bundle 1 essentially done. Next: Bundle 2 (Solve & numbers).
- **0i** ◑ (partial) limit pre-pass fixes: positive *rational* leading
  coefficient was misread as negative (`limit((x^2+x)/2,x,inf)` gave minf →
  now inf) via `coeff_positive`; rational `ndeg>ddeg` now carries the
  leading-ratio sign (`-x^3/(x+1)→minf`). The remaining cases are Gruntz
  omega-rewrite-internals bugs — `1-1/(x+1)→0`, `2-3/(x^2+1)→-1`,
  `2-(1/2)^x→0` — deferred to a **limit-engine hardening** task (patching
  Gruntz internals risks the many working exp/log limits).
- **0j** ✅ `simplify_power` folds `(n/d)^e` for `|e|>=1` (was `>=2`), so the
  reciprocal `(1/2)^(-1)=2`, `1/(2/3)=3/2`, `3/(1/2)=6`. Also cleaned up the
  symbolic `linsolve` fraction forms as a side benefit.
- **0f** ⏭️ the `simplified`-flag early-return is ineffective (timeout is in
  `expand`'s 4097-term squaring, not simplify recursion) AND unsafe (flag not
  perfectly reliable — broke an integrate test); reverted. Real fix = route
  polynomial `expand` through the poly crate / hash-consing (infra).
- **0h** ◑ `resolve_plugin_path` now finds `libmaxima_<name>.<ext>` in
  target/{release,debug} + search dirs: `load_plugin("specfun")` works
  (`bessel_j(0,1.0)=0.7652`), `load_plugin("orthopoly")` works. Parser
  `panic!`-on-bad-input (incl. `,numer` ev-modifier) deferred to a Result-based
  parser task.
- **0c** ✅ improper integrals no longer leak `inf`: any infinite-bound
  candidate still containing inf/minf/und (failed limit, e.g. unresolved
  `atan(inf/√2)`) → noun; a 4-arg definite that falls through returns the
  definite noun, not the indefinite antiderivative. Working cases (`%pi`,
  `√π/2`, …) unchanged. (Proper rational-improper evaluation = Bundle 4 / 2e
  contour engine.)
- **0b** ✅ `eval_sum` infinite-bound gate: convergent numeric geometric (ratio
  by exact sampling, |r|<1) → exact value; divergent/non-geometric/symbolic →
  noun (was substituting `inf` → garbage `1-1/(1+inf)`, `inf*(1+inf)/2`).
- **Found en route (new Tier-0 follow-ups):** `gruntz_limit` wrong on
  `limit(2-(1/2)^x,x,inf)`→0, `limit(x*(x+1)/2,x,inf)`→minf (0i); `1/(1/2)`
  doesn't simplify to 2 — reciprocal of a rational (0j). Both deferred; they're
  why 0b uses exact sampling rather than the limit engine.
- **0a** ✅ exact symbolic Gauss–Jordan in `eval_linsolve` (was f64,
  `to_f64(e).unwrap_or(0.0)` zeroed symbolic RHS → `[x=0,y=0]`). Now correct;
  singular→noun. (`solve(a*x=b)` symbolic-linear + fuller ratsimp deferred.)
- **3c** ✅ special-function numerics in `specfun_num.rs`: `zeta` (exact
  even values %pi^2/6,%pi^4/90,…; Euler–Maclaurin numeric: zeta(3.0)=1.20206),
  `lambert_w` (Halley: 1.0→0.56714), `polylog` (Li_2(1)=%pi^2/6, series numeric).
  f64; arbitrary precision follows a bigfloat backend.

## Bundle 2 self-contained items DONE (1a-i/ii, 1b, 3a, 3b, 3c, 3d).
- **3b** ✅ now complete: eigenvectors for irrational/complex eigenvalues. Exact
  null space of M−λI for every radical eigenvalue; where the divide-based RREF
  leaves an unreducible 1/λ residue, an adjugate column (polynomial in λ,
  reduces under expand) supplies the eigenvector. correct-or-noun.
- **1b** ✅ now complete: `realroots` returns exact rationals (Maxima
  `[x = r, …]`). Factor over Q → linear factors exact; each irreducible factor's
  real roots isolated by Sturm bisection in exact BigRational arithmetic within
  a rational eps. No f64 in the result.

- **1a** ✅ now complete (Cardano/Ferrari): general cubic via Cardano —
  depressed t³+pt+q, real radicals when D≥0, complex radicals (u, v=−p/(3u))
  when D<0 (casus irreducibilis, 3 real roots). General quartic via Ferrari —
  resolvent cubic 8t³+8pt²+(2p²−8r)t−q² supplies t₀ (q=0 falls back to
  biquadratic-in-y). Foundation: a Complex64 verifier (expr_to_complex) checks
  |p(r)|<1e-6 for every root, real or complex; a failed root → noun.

- **1c** ✅ now complete: arbitrary-precision bigfloat backend (astro-float).
  New `Expr::BigFloat` core atom stores a precision-tagged decimal (core keeps
  no bignum-float dep — all compute is in eval). `bfloat(expr)` evaluates the
  whole argument at fpprec digits (constants, arithmetic, powers, elementary
  functions); a bigfloat in +/*/^ folds at the widest operand precision
  (contagion). Display in Maxima `…bN` notation.

- **RootOf** ✅ now complete: `solve` returns `rootof(p,x,k)` nouns for numeric
  univariate factors with no radical solution (general quintic+). All roots via
  Durand–Kerner, ordered real-first then complex; `float` evaluates to the k-th
  root (real `Float` or `a+b·%i`), `bfloat` refines real roots to fpprec digits
  via Newton in astro-float. Solvable factors still return verified radicals
  (mixed polynomials return radicals for the solvable part, rootof for the rest).

## Bundle 2 COMPLETE — all items shipped. Next: Bundle 3 (Summation completion).
