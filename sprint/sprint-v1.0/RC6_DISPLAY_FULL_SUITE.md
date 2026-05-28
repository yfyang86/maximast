# RC6 — Display Engine + Full Test Suite

**Goal:** Implement 2D ASCII display, TeX output, and achieve broad
test-suite compatibility. Target: 60+ rtest files passing.

**Exit criteria:** 2D display matches Maxima output conventions;
`tex(expr)` produces valid LaTeX; majority of rtest suite passes.

---

## Sprint 6.1 — 2D ASCII Display

**Duration:** 3 weeks

### Tasks

- [ ] 2D rendering engine:
  - Fractions displayed vertically: numerator/line/denominator
  - Exponents raised: `x^2` shown with `2` above
  - Matrices with brackets and alignment
  - Square roots with radical sign `sqrt` rendering
  - Sums and products with sigma/pi notation (if terminal supports)
  - Parenthesization based on operator precedence
- [ ] Display modes:
  - `display2d: true/false` — toggle 2D vs 1D
  - `ldisp(expr)` — display with label
  - `disp(expr)` — display without label
  - `grind(expr)` — display in re-parseable 1D format
  - `string(expr)` — return 1D string representation
- [ ] Line width handling:
  - `linel: 79` — max output width
  - Line breaking for long expressions
  - Continuation for wide fractions/matrices
- [ ] Special symbol display:
  - Greek letters in capable terminals
  - Infinity symbol
  - `%pi`, `%e`, `%i` rendering

### Tests

```
#[test] fn display_fraction() {
    // 1/(x+1) should display as:
    //       1
    //     -----
    //     x + 1
}
#[test] fn display_expt() {
    //      3
    // (x+1)
}
#[test] fn display_matrix() {
    //  [ 1  2 ]
    //  [ 3  4 ]
}
#[test] fn display_nested() {
    //         2
    //        x  + 1
    //       --------
    //        x - 1
}
#[test] fn grind_roundtrip() {
    // grind output parses back to equivalent expression
}
```

---

## Sprint 6.2 — TeX Output

**Duration:** 2 weeks

### Tasks

- [ ] `tex(expr)` — generate LaTeX string:
  - Fractions: `\frac{a}{b}`
  - Exponents: `x^{2}`
  - Square roots: `\sqrt{x}`
  - Sums: `\sum_{i=1}^{n}`
  - Integrals: `\int_{a}^{b} f(x) \, dx`
  - Matrices: `\begin{pmatrix} ... \end{pmatrix}`
  - Greek: `\alpha`, `\beta`, etc.
  - Special: `\pi`, `e`, `i`
- [ ] `tex(expr, filename)` — write TeX to file
- [ ] `tex(expr, false)` — return as string without printing
- [ ] `texput(sym, texrep)` — register custom TeX representation

### Tests

```
#[test] fn tex_fraction()  { tex("1/(x+1)") == "\\frac{1}{x+1}" }
#[test] fn tex_expt()      { tex("x^2")     == "x^{2}" }
#[test] fn tex_sqrt()      { tex("sqrt(x)") == "\\sqrt{x}" }
#[test] fn tex_sum()       { tex("sum(i^2, i, 1, n)") == "\\sum_{i=1}^{n}{i^{2}}" }
#[test] fn tex_matrix()    { tex("matrix([a,b],[c,d])") contains "pmatrix" }
#[test] fn texput_custom() { run("texput(hbar, \"\\\\hbar\"); tex(hbar);") contains "\\hbar" }
```

---

## Sprint 6.3 — Symbolic Differentiation (Full)

**Duration:** 3 weeks

### Tasks

- [ ] `diff(expr, var)` — full symbolic differentiation:
  - Power rule, product rule, quotient rule, chain rule
  - Trig: `diff(sin(x),x)` → `cos(x)`, etc.
  - Exponential: `diff(exp(x),x)` → `exp(x)`
  - Logarithmic: `diff(log(x),x)` → `1/x`
  - Inverse trig: `diff(atan(x),x)` → `1/(1+x^2)`
  - Implicit: `diff(f(x),x)` → `'diff(f(x),x)` (noun form)
- [ ] `diff(expr, var, n)` — nth derivative
- [ ] `depends(f, x)` — declare dependencies
- [ ] `gradef(f(x), df)` — define custom derivative
- [ ] Noun/verb distinction:
  - `'diff(f(x),x)` — unevaluated (displayed with d/dx notation)
  - `diff(f(x),x)` — evaluated if possible
- [ ] `at(expr, var=val)` — evaluate at a point

### Tests

```
#[test] fn diff_power()     { diff("x^5", "x")     == "5*x^4" }
#[test] fn diff_product()   { diff("x*sin(x)", "x") == "sin(x)+x*cos(x)" }
#[test] fn diff_chain()     { diff("sin(x^2)", "x") == "2*x*cos(x^2)" }
#[test] fn diff_exp()       { diff("exp(3*x)", "x") == "3*exp(3*x)" }
#[test] fn diff_log()       { diff("log(x)", "x")   == "1/x" }
#[test] fn diff_implicit()  { diff("f(x)", "x")     == "'diff(f(x),x)" }
#[test] fn diff_nth()       { diff("x^4", "x", 3)   == "24*x" }
#[test] fn diff_at()        { run("at(diff(x^3,x), x=2);") == "12" }
```

---

## Sprint 6.4 — Broad rtest Compatibility

**Duration:** 4 weeks

### Tasks

- [ ] Systematic rtest pass-rate tracking:
  - Create compatibility matrix: file × pass/fail/skip
  - Prioritize by dependency (lower numbers first)
- [ ] Implement missing features discovered from rtest failures:
  - `subst`, `psubst`, `lsubst` — substitution variants
  - `apply`, `map`, `maplist`, `fullmap` — mapping functions
  - `lhs`, `rhs` — equation parts
  - `part`, `inpart` — expression parts
  - `args` — list of arguments
  - `op` — operator of expression
  - `ordergreat`, `orderless` — custom ordering
  - `radcan` — radical simplification
  - `logcontract`, `logexpand` — logarithm manipulation
  - `trigexpand`, `trigreduce`, `trigsimp` — trig manipulation
  - `rectform`, `polarform` — complex number forms
  - `realpart`, `imagpart`, `cabs`, `carg` — complex parts
  - `solve(eqn, var)` — basic equation solving (linear, quadratic)
- [ ] Run all 99 rtest files, report pass rates
- [ ] Target: ≥60 files fully passing

### Tests

```
#[test] fn rtest_compatibility() {
    let results = run_all_rtests();
    assert!(results.passing_files >= 60);
    for file in CRITICAL_FILES {
        assert!(results.is_passing(file));
    }
}
```

### Critical files (must pass):

- `rtest1.mac` – `rtest16.mac` (core functionality)
- `rtest1a.mac` (additional core)
- `rtest_abs.mac`
- `rtest_boolean.mac`
- `rtest_equal.mac`
- `rtest_dot.mac`
- `rtest_algebraic.mac`

---

## Deliverable

```
$ maxima-kernel
(%i1) diff(sin(x)*exp(x), x);
                           x            x
(%o1)               cos(x) %e  + sin(x) %e
(%i2) tex(%);
$$\cos x\,e^{x}+\sin x\,e^{x}$$
(%o2)                        false
(%i3) factor(x^6 - 1);
                                 2           2
(%o3)       (x - 1) (x + 1) (x  - x + 1) (x  + x + 1)

Test suite: 67/99 rtest files passing (68%)
```
