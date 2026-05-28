# RC3 — Assumption Database + Comparison Engine (Revised)

**Goal:** Implement the `assume`/`forget`/`is`/`asksign` system so the
kernel can reason about sign, domain, and ordering of symbolic expressions.

**Mathematical foundation:** The assumption database is a constraint
propagation system over a partial order of symbolic expressions. It
maintains a DAG of known relations and infers new facts via transitivity
and algebraic rules (e.g., `x>0 ∧ y>0 ⟹ x*y>0`).

---

## Sprint 3.1 — Fact Database + Sign Inference

**Duration:** 3 weeks

### Algorithm

The fact database stores inequalities as directed edges in a relation
graph. Sign inference follows these rules:

```
sign(n)         = pos/neg/zero    (numeric)
sign(x)         = lookup(db, x)   (symbolic)
sign(x + y)     = pos if sign(x)=pos ∧ sign(y)=pos
sign(x * y)     = sign(x) ⊗ sign(y)   (sign multiplication table)
sign(x^n)       = pos if n even, sign(x) if n odd
sign(abs(x))    = pos|zero
sign(exp(x))    = pos
```

Sign multiplication table:
```
    pos  neg  zero  pnz
pos  pos  neg  zero  pnz
neg  neg  pos  zero  pnz
zero zero zero zero  zero
pnz  pnz  pnz  zero  pnz
```

### Tasks

- [ ] `Fact` type: `{ lhs: Expr, rel: Relation, rhs: Expr }`
- [ ] `FactDatabase`: stores facts, supports query
- [ ] `assume(pred)` — add fact, check consistency
- [ ] `forget(pred)` — remove fact
- [ ] `facts()` — list current assumptions
- [ ] `is(pred)` — evaluate predicate against database
  - Numeric: compute directly
  - Symbolic: query database + inference
  - Compound: decompose `and`/`or`/`not`
- [ ] `asksign(expr)` — determine sign from database
  - Recursive descent through expression structure
  - Apply sign multiplication table
  - Handle `abs`, `exp`, `log` specially
- [ ] `sign(expr)` — internal sign computation

### Tests

```
assume(x>0); is(x>0);       → true
assume(x>0); asksign(x);    → pos
assume(x>0,y>0); asksign(x*y); → pos
assume(x>0); asksign(x^2);  → pos
asksign(exp(x));             → pos
assume(x>0); is(x>=0);      → true
forget(x>0); is(x>0);       → unknown
```

---

## Sprint 3.2 — Contexts + Transitivity

**Duration:** 2 weeks

### Algorithm

Contexts form a tree. Each context inherits facts from its parent.
Fact query walks up the context chain. Transitivity: if `a < b` and
`b < c` then `a < c`. Implementation via transitive closure on query
(not precomputed, to keep inserts fast).

### Tasks

- [ ] Context tree: `initial` → user-created contexts
- [ ] `newcontext(name)`, `supcontext(name, parent)`
- [ ] `activate(ctx)` / `deactivate(ctx)`
- [ ] `killcontext(ctx)`
- [ ] `assuming(pred, expr)` — temporary context
- [ ] Transitive inference for inequality chains
- [ ] `featurep(x, prop)` with `declare(x, integer)` etc.
- [ ] Domain declarations: `integer`, `rational`, `real`, `complex`
- [ ] Property declarations: `even`, `odd`, `positive`, `negative`

---

## Sprint 3.3 — Sign-Aware Simplification

**Duration:** 2 weeks

### Algorithm

Integrate sign information into the simplifier. Key rules:

```
abs(x) → x        when sign(x) = pos
abs(x) → -x       when sign(x) = neg
sqrt(x^2) → x     when sign(x) = pos
sqrt(x^2) → -x    when sign(x) = neg
sqrt(x^2) → abs(x) otherwise
log(exp(x)) → x   when x is real
```

The simplifier queries the assumption database during simplification,
enabling context-dependent reduction.

### Tasks

- [ ] Simplifier hooks into assumption database
- [ ] `abs` simplification with sign info
- [ ] `sqrt` simplification with sign info
- [ ] `log`/`exp` simplification with domain info
- [ ] `max`/`min` simplification with ordering info
- [ ] Conditional simplification: `if x>0 then ...` in known context

### Tests

```
assume(x>0); abs(x);        → x
assume(x<0); abs(x);        → -x
assume(x>0); sqrt(x^2);     → x
declare(x, real); log(exp(x)); → x
```

---

## Sprint 3.4 — rtest Compatibility

**Duration:** 2 weeks

### Tasks

- [ ] Pass `rtest_ask1.mac`
- [ ] Pass `rtest_boolean.mac`
- [ ] Pass `rtest_equal.mac`
- [ ] Backfill rtest1-4 regressions
