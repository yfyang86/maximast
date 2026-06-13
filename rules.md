# LLM Developer Rules — Maxima Kernel (Rust)

Rules, patterns, and hard-won lessons for AI-assisted development on this
codebase. Written for LLM agents (Claude, GPT, Codex, etc.) working on
the Maxima Rust kernel.

---

## 1. Project Shape

```
maxima-kernel/          ~24K lines of Rust
├── crates/core/        Expr enum, Operator, symbol interning (read-only, rarely changes)
├── crates/parser/      Lexer + Pratt parser (change when adding syntax)
├── crates/poly/        Polynomial ring, GCD, factoring (math library, self-contained)
├── crates/eval/        Evaluator — where 90% of work happens
│   ├── src/eval.rs     Core dispatch (~7K lines) — THE hot file
│   ├── src/help.rs     Built-in help(...) documentation system
│   ├── src/help.toml   Embedded TOML help pages
│   ├── src/integrate.rs  Integration engine (extracted from eval.rs)
│   ├── src/simp.rs     Simplifier (canonical forms)
│   ├── src/complex.rs, sets.rs, strings.rs, numtheory.rs, ...  (domain modules)
│   └── tests/          Integration tests (~20 test files, 1100+ tests)
├── crates/repl/        REPL binary with tab completion
└── walkthrough/        41 .mac tutorial scripts
```

**The key file is `crates/eval/src/eval.rs`.** It contains a giant match
on function names (~200 arms). New functions go here or in a domain module.

---

## 2. How to Add a New Function

This is the most common task. Follow this exact pattern:

### Step 1: Choose or create a module

If the function fits an existing domain (sets, strings, numtheory, etc.),
add it there. Otherwise create `crates/eval/src/mymodule.rs`.

### Step 2: Write the function

```rust
// In mymodule.rs
pub(crate) fn eval_myfunc(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "myfunc" => {
            // Implementation here
            Some(result)
        }
        _ => None,
    }
}
```

Return `Option<Expr>`:
- `Some(result)` → function handled, return result
- `None` → function not handled, evaluator returns noun form

### Step 3: Register in lib.rs

```rust
pub mod mymodule;
```

### Step 4: Wire into eval.rs

Find the match in `eval_funcall` and add:
```rust
"myfunc" => {
    if let Some(r) = crate::mymodule::eval_myfunc(&func_name, &evaled_args) {
        return r;
    }
    Expr::call(&func_name, evaled_args)
}
```

**Critical:** Use `&func_name` (not `func_name`) because `func_name` is `String`
and `Expr::call` takes `&str`.

### Step 5: Add tests

Create `crates/eval/tests/mymodule_test.rs`:
```rust
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn my_test() { assert_eq!(run("myfunc(3);"), "expected"); }
```

### Step 6: Add to tab completion

In `crates/repl/src/main.rs`, add to `BUILTIN_FUNCTIONS` array.

### Step 7: Add a help entry

If the function is user-facing, document it in `crates/eval/src/help.toml`:

```toml
[[function]]
name = "myfunc"
alias = ["my_func"]
title = "Short human-readable title"
description = "What it does. Markdown accepted."
usage = """
```
myfunc(x)
myfunc(x, y)
```
"""
arguments = """
- `x`: description of x.
- `y`: description of y.
"""
details = "Extended explanation, edge cases, assumptions."
value = "Description of the return value."
references = ["https://example.com/docs"]
authors = ["Your Name"]
```

Then verify interactively:
```maxima
help("myfunc");
help("myfunc", "usage");
```

---

## 3. Common Pitfalls (learned the hard way)

### 3.1 The Display → re-parse trap

**Never** format an `Expr` to string and re-parse it. The `Display` impl
does NOT round-trip through the parser for `if/for/block/lambda` expressions.
Use `eval_expr_with_env(expr, env)` to evaluate a parsed AST directly.

The old `run_script` did `format!("{};", expr)` then `eval_str_with_env` —
this broke batch mode for any script with control flow.

### 3.2 The simplifier vs evaluator boundary

`simplify()` does algebraic canonicalization (flatten +/*, collect like terms,
power rules). `meval()` does function dispatch and variable lookup.

**Do not call `meval` from `simplify`** — it causes infinite loops.
If you need evaluation in a simplifier rule, return the unsimplified form
and let the evaluator handle it.

`simplify()` is idempotent. `meval()` is not (it has side effects via `env`).

### 3.3 The `func_name` type

`eval_funcall` resolves the function name as `let func_name = resolve(name)`
which returns `String`. All downstream calls need `&func_name` for `&str`.
Using `func_name` directly in `Expr::call(func_name, ...)` fails with
"expected `&str`, found `String`".

### 3.4 Borrow checker and Environment

You cannot borrow `env.some_field` immutably while passing `env` mutably
to a function. Clone the field first:

```rust
// WRONG: borrow conflict
let result = compute(&env.data, env);

// RIGHT: clone first
let data = env.data.clone();
let result = compute(&data, env);
```

### 3.5 sqrt simplification must happen in THREE places

1. **Evaluator** (`eval_math_func`): `sqrt(4)` → `2` for integer args
2. **Simplifier** (`simplify_power`): `sqrt(x)^2` → `x`
3. **Integrator** (`normalize_sqrt_powers`): `sqrt(x)` → `x^(1/2)` before power rule

Missing any one of these causes subtle bugs where some paths produce
`sqrt(4)` instead of `2`.

### 3.6 Formulas MUST be verified numerically

**Every** integration formula, trig identity, or algebraic transformation
must be verified by numerical evaluation at 2-3 test points. We had THREE
Euler substitution formulas that were all wrong — they passed structural
tests but failed numerical checks. The formulas were removed entirely.

Pattern: compute `F(1) - F(0)` from your formula, compare against
numerical integration via Python/trapezoidal rule.

### 3.7 abs() in limits needs derivative sign

`limit(f(x)/abs(g(x)), x, a)` cannot use direct substitution because
`abs` is not differentiable at zero. The fix:
1. Evaluate `g(a)` — if nonzero, use sign directly
2. If `g(a) = 0`, compute `g'(a)` to determine sign near `a`
3. For bidirectional limits, compute both sides and return `und` if they disagree

The naive approach (`direction > 0 → abs(g) = g`) fails when `g(x) = 1-x`
approaching from above (where `g < 0`).

### 3.8 Match arm ordering matters

In `simplify_power`, the `(a^b)^c` arm matches ANY `MExpt` base with
integer exponent. If you add a more specific arm like `(x^(1/2))^n`,
it MUST come before the general arm or it's unreachable.

---

## 4. Testing Strategy

### 4.1 Test hierarchy

1. **Unit tests** in `crates/eval/src/eval.rs` `mod tests` — basic function behavior
2. **Integration tests** in `crates/eval/tests/*.rs` — domain-specific suites
3. **Walkthroughs** in `walkthrough/*.mac` — batch-mode end-to-end scripts
4. **Stress tests** in `tests/stress_test.rs` — edge cases and regressions

### 4.2 Writing effective tests

```rust
// Good: exact match for deterministic outputs
assert_eq!(run("sin(%pi/6);"), "1/2");

// Good: structural check when exact output varies
assert!(r.contains("log") && !r.contains("integrate"), "got: {}", r);

// Bad: float comparison (fragile)
assert_eq!(run("float(sqrt(2));"), "1.414213562373095");  // may vary
```

### 4.3 When Display changes break tests

Changing the `Display` impl (e.g., `x^(-1)` → `1/x`) breaks MANY tests.
Search for the old pattern across all test files:
```sh
grep -rn 'old_pattern' crates/eval/tests/ crates/eval/src/eval.rs
```

### 4.4 The walkthrough smoke test

```sh
for f in walkthrough/*.mac; do
    cargo run --bin maxima-repl -- -b "$f" >/dev/null 2>&1 || echo "FAIL: $f"
done
```

This catches panics, parse errors, and regressions. Run after any change.

---

## 5. Architecture Decisions

### 5.1 Why modules, not one big file

`eval.rs` was 10K lines. We extracted:
- `integrate.rs` (3K) — integration engine
- `complex.rs`, `sets.rs`, `strings.rs`, etc. — domain modules

Each module exports `pub(crate) fn eval_xxx(name, args) -> Option<Expr>`.
The evaluator dispatches to these via match arms. This keeps `eval.rs`
under 7K lines and makes each domain independently readable.

### 5.2 Why `Option<Expr>` not `Expr`

Domain functions return `Option<Expr>`:
- `Some(result)` — handled successfully
- `None` — can't handle this input (wrong types, missing args)

The evaluator falls back to noun form when `None` is returned.
This avoids panics and makes partial implementations safe.

### 5.3 Why separate `Operator::MSet` for sets

Sets `{1,2,3}` could be stored as sorted lists. But having a distinct
`MSet` operator means:
- Display uses `{...}` not `[...]`
- Set operations can check `op == MSet` cheaply
- No confusion between `[1,2,3]` (list) and `{1,2,3}` (set)

### 5.4 Why `NativeFn` plugin API

```rust
pub type NativeFn = fn(&[Expr], &mut Environment) -> Expr;
```

This lets Rust code register functions callable from Maxima.
Dispatch order: native → user-defined → lambda → autoload → noun form.
Native functions survive `kill(all)`.

Future: dynamic loading via `dlopen` for `.so` plugins.

---

## 6. Code Review Checklist

When reviewing changes to this codebase:

- [ ] Does every new formula have a numerical verification test?
- [ ] Does the change touch `Display`? If so, search for broken test expectations.
- [ ] Does any `simplify()` call risk infinite recursion?
- [ ] Are match arms in the correct order (specific before general)?
- [ ] Is `func_name` used as `&func_name` in `Expr::call`?
- [ ] Does `sqrt(n)` simplify for integer `n` in all three layers?
- [ ] Are `i64` multiplications checked for overflow where inputs can be large?
- [ ] Does the feature work in batch mode (`-b`) not just REPL?
- [ ] Is the function added to tab completion?
- [ ] Is the function documented in `crates/eval/src/help.toml` (if user-facing)?
- [ ] Is there a walkthrough `.mac` script demonstrating the feature?

---

## 7. Sprint Methodology

We use numbered sprints (S1, S2, ...) grouped into phases:

1. **Plan**: identify gap, estimate size (Small/Medium/Large), assess risk
2. **Implement**: write module, wire into eval.rs, build
3. **Test**: run existing tests, add new tests, run walkthroughs
4. **Document**: add walkthrough, update README/user-manual if needed
5. **Commit**: one commit per sprint with descriptive message
6. **Push**: push to feature branch, PR to dev

Sprint sizes:
- **Small** (~1-2h): new functions in existing module, table entries
- **Medium** (~3h): new module with algorithm implementation
- **Large** (~5h+): new subsystem (plotting, pattern matching, ODE solver)

---

## 8. Expression Patterns

### Common Expr constructors
```rust
Expr::int(42)                           // Integer
Expr::sym("x")                          // Symbol
Expr::Float(3.14)                       // Float
Expr::Rational { num: 1, den: 2 }      // 1/2
Expr::add(a, b)                         // a + b
Expr::mul(a, b)                         // a * b
Expr::pow(a, b)                         // a ^ b
Expr::neg(a)                            // -a  (= -1 * a)
Expr::div(a, b)                         // a / b (= a * b^(-1))
Expr::sub(a, b)                         // a - b (= a + (-1)*b)
Expr::call("sin", vec![x])             // sin(x)
Expr::list(vec![a, b, c])              // [a, b, c]
Expr::set(vec![a, b, c])               // {a, b, c}
```

### Matching patterns
```rust
// Match a function call
if let Expr::List { op: Operator::Named(id), args, .. } = expr {
    let fname = resolve(*id);
    // ...
}

// Match a sum
if let Expr::List { op: Operator::MPlus, args, .. } = expr { ... }

// Match a specific symbol
let pi_id = intern("%pi");
if let Expr::Symbol(id) = expr { if *id == pi_id { ... } }
```

### Helper functions
```rust
contains_var(expr, var)     // does expr contain var?
subst(new, old, expr)       // substitute new for old in expr
to_i64(expr)                // try to extract i64 from Integer
to_f64(expr)                // try to extract f64 from any numeric
simplify(expr)              // algebraic simplification
expand(expr)                // polynomial expansion
ratsimp(expr)               // rational simplification
```

---

## 9. Lessons from This Project

1. **Numerical verification beats structural testing.** A formula can look
   right and pass pattern tests but be mathematically wrong. Always check
   `d/dx[F(x)] = f(x)` numerically for integration formulas.

2. **Refactor before the file hits 10K lines.** We extracted `integrate.rs`
   at 10K — should have done it at 5K. Each module should be <1K lines.

3. **The Display impl is load-bearing.** Changing how expressions print
   breaks tests across the entire codebase. Plan for 30+ test updates.

4. **Pattern matching order is a footgun.** Rust match arms are tried in
   order. A general arm shadows all specific arms below it.

5. **The simplifier and evaluator must stay separate.** Mixing them causes
   infinite loops. The simplifier transforms structure; the evaluator
   computes values.

6. **"Return noun form" is always safe.** When in doubt, return the
   unevaluated expression. Wrong answers are worse than no answers.

7. **Clone before borrow.** The borrow checker will fight you when
   `Environment` is both read and written. Clone the field you need to read,
   then pass `env` mutably.

8. **Batch mode is stricter than REPL.** Test with `-b` not just `-e`.
   The batch runner hits code paths the single-expression evaluator doesn't.

9. **Three layers of sqrt.** Every "simple" feature may need changes in
   the evaluator, simplifier, AND integrator. Missing one layer creates
   subtle bugs that only appear in specific contexts.

10. **Sprint small, test often.** Each sprint should be independently
    testable and committable. Never accumulate 3 sprints of untested changes.
