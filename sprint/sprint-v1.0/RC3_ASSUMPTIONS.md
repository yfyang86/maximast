# RC3 — Assumption Database + Comparison Engine

**Goal:** Implement the `assume`/`forget`/`is` system and comparison logic
so the kernel can reason about sign, domain, and ordering of symbolic
expressions. Pass `rtest_ask1.mac` and `rtest_boolean.mac`.

**Exit criteria:** `assume(x>0); is(x>0);` → `true`;
context save/restore works; boolean simplification works.

---

## Sprint 3.1 — Fact Database

**Duration:** 3 weeks

### Tasks

- [ ] Fact storage model:
  - Facts are inequalities/equalities involving symbolic expressions
  - Stored per-context (contexts can be nested/stacked)
  - Representation: `Fact { lhs: Expr, rel: Relation, rhs: Expr }`
  - Relations: `LessThan`, `LessEqual`, `Equal`, `GreaterEqual`, `GreaterThan`, `NotEqual`
- [ ] `assume(pred)` — add a fact to the current context
  - Validate: not contradictory with existing facts
  - Return `redundant` if already known
  - Return `inconsistent` if contradicts
- [ ] `forget(pred)` — remove a fact
- [ ] `facts()` — list current assumptions
- [ ] `is(pred)` — query whether a predicate follows from assumptions
  - Return `true`, `false`, or `unknown`
- [ ] `asksign(expr)` — determine sign of expression from assumptions
  - Return `pos`, `neg`, `zero`, or `pnz` (unknown)
  - For numeric expressions: compute directly
  - For symbolic: consult fact database

### Tests

```
#[test] fn assume_basic()      { run("assume(x>0); is(x>0);")    == "true" }
#[test] fn assume_derived()    { run("assume(x>0); is(x>=0);")   == "true" }
#[test] fn assume_unknown()    { run("is(y>0);")                  == "unknown" }
#[test] fn forget_basic()      { run("assume(x>0); forget(x>0); is(x>0);") == "unknown" }
#[test] fn facts_list()        { run("assume(x>0, y<1); facts();") == "[x>0, y<1]" }
#[test] fn assume_redundant()  { run("assume(x>0); assume(x>0);") == "redundant" }
#[test] fn asksign_numeric()   { run("asksign(5);")  == "pos" }
#[test] fn asksign_symbolic()  { run("assume(x>0); asksign(x);") == "pos" }
#[test] fn asksign_product()   { run("assume(x>0, y>0); asksign(x*y);") == "pos" }
```

---

## Sprint 3.2 — Contexts

**Duration:** 2 weeks

### Tasks

- [ ] Context model:
  - Named contexts: `initial` (default), user-created
  - `newcontext(name)` — create and activate a context
  - `supcontext(name, parent)` — create sub-context
  - `activate(ctx)` / `deactivate(ctx)` — push/pop
  - `killcontext(ctx)` — remove context and all its facts
  - Facts in active contexts are all visible
- [ ] `block` and function calls respect context scoping
- [ ] `assuming(pred, expr)` — temporarily assume, evaluate, then restore

### Tests

```
#[test] fn context_new()   { run("newcontext(c1); assume(x>0); is(x>0);") == "true" }
#[test] fn context_kill()  { run("newcontext(c1); assume(x>0); killcontext(c1); is(x>0);") == "unknown" }
#[test] fn assuming_temp() { run("assuming(x>0, is(x>0));") == "true" }
#[test] fn assuming_no_leak() { run("assuming(x>0, is(x>0)); is(x>0);") == "unknown\ntrue" }
```

---

## Sprint 3.3 — Boolean Simplification

**Duration:** 2 weeks

### Tasks

- [ ] Boolean operators: `and`, `or`, `not`
- [ ] Boolean simplification rules:
  - Short-circuit: `true and x → x`, `false and x → false`
  - Identity: `true or x → true`, `false or x → x`
  - Double negation: `not not x → x`
  - De Morgan: when beneficial for canonicalization
- [ ] `is(pred)` integration with boolean logic:
  - `is(x>0 and y>0)` decomposes into `is(x>0) and is(y>0)`
  - `is(not pred)` → negate result
- [ ] Comparison simplification:
  - `x > x → false`
  - `x = x → true`
  - Numeric comparisons evaluated directly
- [ ] Predicate functions: `evenp`, `oddp`, `primep`

### Tests

```
#[test] fn bool_and_true()   { simplify("true and x>0") == "x>0" }
#[test] fn bool_or_true()    { simplify("true or x>0")  == "true" }
#[test] fn bool_not_not()    { simplify("not not p")     == "p" }
#[test] fn is_compound()     { run("assume(x>0, y>0); is(x>0 and y>0);") == "true" }
#[test] fn compare_same()    { run("is(x=x);")          == "true" }
#[test] fn compare_numeric() { run("is(3 > 2);")         == "true" }
```

---

## Sprint 3.4 — rtest Compatibility

**Duration:** 2 weeks

### Tasks

- [ ] Run `rtest_ask1.mac` and fix failures
- [ ] Run `rtest_boolean.mac` and fix failures
- [ ] Run `rtest_equal.mac` and fix failures
- [ ] Backfill any `rtest1–4` regressions
- [ ] Implement `declare(x, integer)`, `declare(x, real)`, etc.
  - Domain declarations: integer, rational, real, complex
  - Property declarations: even, odd, prime
- [ ] `featurep(x, prop)` — check declared properties

### Tests

```
#[test] fn rtest_ask1()     { assert_rtest_passes("tests/rtest_ask1.mac"); }
#[test] fn rtest_boolean()  { assert_rtest_passes("tests/rtest_boolean.mac"); }
#[test] fn rtest_equal()    { assert_rtest_passes("tests/rtest_equal.mac"); }
#[test] fn declare_integer(){ run("declare(n, integer); featurep(n, integer);") == "true" }
```

---

## Deliverable

```
$ maxima-kernel
(%i1) assume(x > 0, y > 0);
(%o1)                      [x>0, y>0]
(%i2) is(x*y > 0);
(%o2)                         true
(%i3) assuming(z < 0, is(z^2 > 0));
(%o3)                         true
(%i4) is(z > 0);
(%o4)                        unknown
```
