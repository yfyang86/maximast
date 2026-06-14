# Maxima Rust Kernel v8.0 — Deepen the Symbolic Core

## Theme

Push the **differential / difference-algebra layer** — the one place where
Maxima is structurally weakest, where FriCAS leads, and where no Rust
competitor exists. This is the project's strategic differentiator: the
polynomial/number core can (in principle) be matched by wrapping mature C
libraries, but a complete, native, permissively-licensed integration and
summation tower is genuinely novel.

This plan is the direct continuation of the recommendations in
[`research/survey/ALGORITHM_SURVEY.md`](../../research/survey/ALGORITHM_SURVEY.md)
(Phases 2–4 there), scoped to what is achievable on top of the kernel as it
stands today.

> **Version note.** `Cargo.toml` reads `5.0.0`, but commit history references
> "V6.0" (AC pattern matching, residues, advanced trig, ODE) and "V7.0"
> (dynamic Rust plugins) as already done; the last *sprint document* is
> `sprint-v5.0`. This plan is labelled **v8.0** to sit unambiguously after the
> V6/V7 work. Bump `Cargo.toml` to match when the first sprint lands.

## Current State (audited 2026-06-14)

- ~31k lines Rust, **1117 tests**, ~290 dispatched functions, 41 walkthroughs.
- Integration cascade (`integrate.rs`, 3.1k lines): table → linearity →
  power rule → Hermite reduction → partfrac → u-substitution → by-parts →
  algebraic factoring over Q(√d) → Risch-Norman heuristic → Risch tower.
- **Risch tower** (`risch_tower.rs`, `risch_integrate.rs`): builds log/exp
  differential-field towers; integrates polynomial-in-`t` integrands and a
  **constant-coefficient-only** Risch differential equation.
- **Summation**: Gosper (indefinite) + Zeilberger creative telescoping
  (`zeilberger.rs`) for definite hypergeometric sums.
- **Limits**: Gruntz MRV (`gruntz.rs`, 823 lines) on a limited series engine
  (`series.rs`, 261 lines).

### What the core cannot yet do (the targets)

| Input | Today | Should be |
|-------|-------|-----------|
| `integrate(1/(x^3+1), x)` log part | ad-hoc partfrac, fragile on irreducible factors | Rothstein–Trager–LRT, always correct |
| `integrate(1/log(x), x)` | noun form | `li(x)` (logarithmic integral; nonelementary, named) |
| `integrate(exp(x^2), x)` | noun form | `(sqrt(%pi)/2)*erfi(x)` (named, provably nonelementary) |
| `integrate(1/sqrt(x^2+2*x+5), x)` | partial | `asinh((x+1)/2)` via Euler substitution |
| `integrate(1/((x+1)*sqrt(x^2+1)), x)` | noun form | log form via Euler substitution (was removed) |
| `integrate(exp(-x^2)*x^2, x, 0, inf)` | special-cased | general via Almkvist–Zeilberger |
| `taylor(tan(x), x, 0, 9)` / Puiseux at branch points | limited | robust Laurent/Puiseux series |

---

## Sprints

| Sprint | Content | Size | Survey ref |
|--------|---------|------|------------|
| **S1** | Lazard–Rioboo–Trager logarithmic part (rational integration) | Medium | §1.1 GO |
| **S2** | Complete Risch differential-equation solver | Large | §1.2 GO |
| **S3** | General Euler substitution `∫R(x,√(ax²+bx+c))` | Medium | §1.3 GO (partial) |
| **S4** | Robust power-series engine (Laurent / Puiseux) | Medium | §2.1 substrate |
| **S5** | Holonomic closure + Almkvist–Zeilberger (definite integrals) | Large | §1.5 / §3.4 GO |
| **S6** | Trager radical-only algebraic integration | Large | §1.3 GO (partial) |
| **S7** | Named nonelementary antiderivatives (li, Ei, erf/erfi, Si, Ci, Fresnel) | Small | enables S2/S5 |

### Phasing

| Phase | Sprints | Focus |
|-------|---------|-------|
| **Phase 1 — Rational+Transcendental completeness** | S7 → S1 → S2 | Make the existing tower correct and decision-complete for elementary transcendental integrands |
| **Phase 2 — Algebraic frontier** | S3 → S6 | The repeatedly-deferred √-integration gap |
| **Phase 3 — Definite & series depth** | S4 → S5 | Reuse the Zeilberger/Ore infrastructure for integrals |

S7 is sequenced first because S2 and S5 need a vocabulary of named
nonelementary functions to return (otherwise "provably nonelementary" inputs
have nothing to map to and must stay as noun forms).

---

## S1 — Lazard–Rioboo–Trager Logarithmic Part (Medium, ~6h)

**Why.** The spine of the whole integration tower (survey §1.1). Today the
log part of `∫P/Q dx` is recovered via partial fractions, which is fragile
when `Q` has irreducible factors of degree ≥ 2 or repeated roots. LRT computes
the logarithmic part directly from a resultant, with no factoring of `Q` over
extensions required.

**Algorithm** (Bronstein, *Symbolic Integration I*, ch. 2):
1. Hermite reduction (already present) leaves a proper rational function with
   square-free denominator.
2. `R(z) = resultant_x(P - z·Q', Q)` — already have `resultant` in `poly`.
3. The distinct roots `z_i` of `R` are the residues; the log argument is
   `gcd(P - z_i·Q', Q)`.

**Tasks**
- [ ] Add `lrt_log_part(p, q, var)` in `crates/poly/src/hermite.rs` (the
      resultant/subresultant machinery already lives there).
- [ ] Wire it into `integrate.rs` *after* Hermite reduction, before the
      partfrac fallback (keep partfrac as fallback when roots aren't rational).
- [ ] Handle the rational-residue case (`z_i ∈ Q`) fully; emit
      `Σ z_i·log(v_i(x))`.

**Verify**
```
integrate(1/(x^3+1), x);            → log + atan, derivative checks to 1/(x^3+1)
integrate((x^2+1)/(x^3-x), x);      → combination of logs
integrate(1/(x^4+1), x);            → unchanged result, now via LRT
```
Success: each result differentiates back to the integrand (numeric check at
3 points), and the existing `rtest_integrate` subset still passes.

---

## S2 — Complete Risch Differential-Equation Solver (Large, ~12h)

**Why.** `solve_risch_de_simple` only handles a **constant** coefficient with
polynomial RHS. The general Risch DE `y' + f·y = g` (with weak normalization,
denominator bounds, and degree bounds) is what makes the transcendental Risch
a genuine decision procedure (survey §1.2). This is the highest-leverage
correctness upgrade in the plan.

**Algorithm** (Bronstein ch. 6: `WeakNormalizer`, `RdeBound`, `SPDE`,
`PolyRischDE`):
- [ ] `weak_normalize(f, D)` — remove the part of the denominator that would
      block a solution.
- [ ] Denominator bound (`RdeBound`) for the exponential and primitive cases.
- [ ] Degree bound + `SPDE` (Rothstein's special polynomial DE) to reduce to a
      bounded polynomial solve.
- [ ] Replace `solve_risch_de_simple` call sites in
      `integrate_exponential` / `integrate_primitive`.
- [ ] **Decision completeness**: when no solution exists in the tower, the
      integrand is provably nonelementary — return the noun form *or* a named
      special function from S7 (e.g. `∫exp(x²) → erfi`).

**Verify**
```
integrate(x*exp(x^2), x);           → exp(x^2)/2
integrate(exp(x^2), x);             → (sqrt(%pi)/2)*erfi(x)   (S7)
integrate(1/log(x), x);             → li(x)                   (S7)
integrate((2*x+1)*exp(x^2+x), x);   → exp(x^2+x)
integrate(x*exp(x)/(x+1)^2, x);     → exp(x)/(x+1)
```
Success: elementary cases differentiate back exactly; nonelementary cases map
to the correct named function and `diff` of the answer recovers the integrand.

**Risk.** This is the largest single sprint. The constant-field zero-test is
undecidable in general — gate it with the existing simplifier + a numeric
fallback, and document the heuristic boundary (every real implementation does
this). If it overruns, ship `WeakNormalizer` + bounded polynomial solve
(covers most user inputs) and defer full `SPDE`.

---

## S3 — General Euler Substitution (Medium, ~5h)

**Why.** `∫R(x, √(ax²+bx+c)) dx` for rational `R` is the single most
requested concrete gap; three special cases were implemented in V4.4 and then
**removed for being numerically wrong** (see `V4_REMAINING.md`). A correct,
general engine retires the gap.

**Algorithm.** Pick the applicable Euler substitution to rationalize:
- `a > 0`: `√(ax²+bx+c) = ±√a·x + t`
- `c > 0`: `√(ax²+bx+c) = x·t ± √c`
- real roots `x₁,x₂`: `√(a(x-x₁)(x-x₂)) = t·(x-x₁)`

Each yields `x` and `dx` as rational functions of `t`; integrate the resulting
rational function (now via S1/Hermite) and back-substitute.

**Tasks**
- [ ] `euler_substitute(integrand, radicand, var)` in a new
      `crates/eval/src/euler.rs`.
- [ ] Detect `√(quadratic)` subexpressions in `integrate.rs`; route to Euler
      before the generic algebraic-factoring path.
- [ ] **Mandatory numeric verification** of every returned formula at 3+
      points (per `skills.md` "Fix a Wrong Formula"); fall back to noun form on
      mismatch rather than risk a repeat of the V4.4 bug.

**Verify**
```
integrate(1/sqrt(x^2+2*x+5), x);        → asinh((x+1)/2)
integrate(1/((x+1)*sqrt(x^2+1)), x);    → log form, diff-checked
integrate(sqrt(x^2-1), x);              → (x*sqrt(x^2-1) - log(x+sqrt(x^2-1)))/2
integrate(x/sqrt(2*x-x^2), x);          → diff-checked
```

---

## S4 — Robust Power-Series Engine (Medium, ~6h)

**Why.** `series.rs` is thin; Gruntz limits and Almkvist–Zeilberger (S5) both
need a dependable series substrate. Upgrading it improves `taylor`, `limit`,
and unblocks S5 (survey §2.1).

**Tasks**
- [ ] Lazy/truncated **Laurent** series type (handle poles, not just Taylor).
- [ ] **Puiseux** series (fractional exponents) for branch points / `sqrt`.
- [ ] Series arithmetic: `+ − × ÷`, composition, `exp`/`log`/`sin`/`cos` of a
      series, reversion.
- [ ] Reuse from `taylor`; feed Gruntz's `rewrite` step (its known weak spot,
      survey §2.1).

**Verify**
```
taylor(tan(x), x, 0, 9);            → x + x^3/3 + 2*x^5/15 + 17*x^7/315 + ...
taylor(1/(exp(x)-1), x, 0, 4);      → 1/x - 1/2 + x/12 - x^3/720 (Laurent)
taylor(sqrt(1+x), x, 0, 4);         → 1 + x/2 - x^2/8 + ...
limit((tan(x)-x)/x^3, x, 0);        → 1/3   (via series, not L'Hopital)
```

---

## S5 — Holonomic Closure + Almkvist–Zeilberger (Large, ~10h)

**Why.** Mathematica handles most *named definite* integrals via creative
telescoping / Meijer-G; the open-source path is Almkvist–Zeilberger on an
Ore-algebra (survey §1.5, §3.4). The kernel already has Zeilberger for *sums* —
this sprint transposes the same machinery to `∂x` for *integrals* and adds the
holonomic closure operations that feed it.

**Tasks**
- [ ] Factor the Ore-operator / creative-telescoping core out of
      `zeilberger.rs` into a shared `crates/eval/src/ore.rs`.
- [ ] Holonomic closure: sum, product, and `∫` of D-finite functions
      (linear ODE for the result from ODEs of the operands).
- [ ] Almkvist–Zeilberger for hyperexponential integrands → linear ODE for the
      parameter integral; solve the ODE (reuse `ode.rs`).
- [ ] Apply to `∫₀^∞` / `∫_{-∞}^∞` families currently special-cased in
      `integrate.rs` (Gaussian-cosine, `x^n·exp(-x)`), generalizing them.

**Verify**
```
integrate(exp(-x^2)*x^2, x, 0, inf);     → sqrt(%pi)/4
integrate(x^n*exp(-x), x, 0, inf);       → factorial(n)   (now via AZ, general n)
integrate(exp(-a*x^2)*cos(b*x), x, 0, inf); → existing result, now general path
```

**Risk.** Large; depends on S4. If it overruns, ship the holonomic-closure +
AZ engine for the hyperexponential case only and defer general D-finite
integrands.

---

## S6 — Trager Radical-Only Algebraic Integration (Large, ~10h)

**Why.** The hardest part of indefinite integration and the clearest place the
project can approach FriCAS (survey §1.3). Scope deliberately limited to a
**single radical of a quadratic or cubic** (pseudo-elliptic basics) — the
general non-radical case is explicitly out of scope.

**Tasks**
- [ ] Represent `Q(x)[y]/(y² − r(x))` using the existing `poly_alg` /
      `alg_field` machinery in the `poly` crate.
- [ ] Integral basis + Hermite reduction on the curve.
- [ ] Logarithmic part via the algebraic analogue of LRT (S1 reused).
- [ ] Decide elementarity; return noun form when the antiderivative is not
      elementary (the common case for genuine elliptic integrals).

**Verify**
```
integrate(1/sqrt(x^3+1), x);     → noun form (correctly: nonelementary elliptic)
integrate(x/sqrt(x^4+1), x);     → (1/2)*asinh(x^2)
integrate((x^2+1)/sqrt(x^2+x), x); → diff-checked elementary result
```

**Risk.** Highest difficulty in the plan. Treat as research-grade; acceptable
outcome is "radical-quadratic cases solved, elliptic cases correctly declared
nonelementary." Do **not** ship any formula that fails numeric verification.

---

## S7 — Named Nonelementary Antiderivatives (Small, ~3h)

**Why.** S2 and S5 need targets for provably-nonelementary integrands. Maxima
ships these as first-class functions; here they are plugin-only or absent.

**Tasks**
- [ ] Register `li`, `Ei`/`expintegral_ei`, `erf`/`erfi`, `Si`/`Ci`,
      `fresnel_s`/`fresnel_c` in the evaluator (definitions, `diff` rules,
      float evaluation, `tex`).
- [ ] `diff` rules so the verification loop closes:
      `diff(li(x),x) = 1/log(x)`, `diff(erf(x),x) = 2*exp(-x^2)/sqrt(%pi)`, etc.
- [ ] Promote the relevant `specfun` plugin entries to built-ins (or document
      that the integrator requires the plugin loaded).

**Verify**
```
diff(li(x), x);          → 1/log(x)
diff(erf(x), x);         → 2*exp(-x^2)/sqrt(%pi)
diff(erfi(x), x);        → 2*exp(x^2)/sqrt(%pi)
diff(Si(x), x);          → sin(x)/x
float(erf(1));           → 0.8427...
```

---

## Success Criteria (whole release)

- `cargo build` zero errors, `cargo test` zero failures (target ≥ 1180 tests).
- Every new integration/AZ formula has a numeric-verification test at ≥ 3
  points (per `skills.md`); **noun form is always preferred over a wrong
  answer.**
- A new `walkthrough/42_risch_algebraic.mac` (and/or `43_definite_integrals.mac`)
  demonstrating the headline results, runnable in batch mode.
- Relevant `rtest_integrate` / `rtest_taylor` / `rtest_limit` cases from the
  bundled Maxima test corpus pass (track count before/after).

## Out of Scope (deferred to a later stage)

| Item | Reason |
|------|--------|
| **Meijer-G table + matching** | Decades of table-building; survey §1.5 says DEFER. The AZ path (S5) covers most named definite integrals without it. |
| **Karr / Schneider ΠΣ summation** | 6–10 PM; a release of its own (survey §3.3). |
| **Risch parametric / Liouvillian (Raab)** | Research-grade; survey §1.6 DEFER. |
| **Reduce / CAD quantifier elimination** | Separate strategic track (the user's "Reduce/CAD frontier" option); doubly-exponential, its own multi-month project. |
| **Multivariate polynomial factoring** | Foundational but orthogonal to the differential-algebra theme; warrants its own release (it gates Tier-2 breadth, not the symbolic-core depth targeted here). |

---

## Topics Needing Your Input

| # | Topic | Question |
|---|-------|----------|
| 1 | **S2 scope** | Full `SPDE`/`PolyRischDE` (true decision procedure, ~12h, risk of overrun) vs. `WeakNormalizer` + bounded polynomial solve (covers most inputs, ~6h)? |
| 2 | **S6 ambition** | Attempt Trager radical-only this release, or defer all algebraic integration and spend the budget making S2/S5 rock-solid? |
| 3 | **Named functions** | Promote `erf`/`Ei`/… to built-ins (always available, duplicates the `specfun` plugin) vs. require the plugin loaded (no duplication, integrator fails without it)? |
| 4 | **bignum dependency** | S1/S6 resultants over Q can overflow `i64`. Stay pure-`num::BigInt` (current, permissive) or is an LGPL `rug`/FLINT-FFI fast path acceptable for the heavy algebraic work? (survey §"Cross-cutting": affects license posture). |
| 5 | **Verification budget** | Is a numeric-only verification gate sufficient, or do you want symbolic `diff`-back assertions required for every returned antiderivative (slower to author, stronger guarantee)? |
