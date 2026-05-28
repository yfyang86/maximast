# Skills Reference — Maxima Kernel Development

Reusable procedures for common development tasks on the Maxima Rust kernel.
Each skill is a step-by-step recipe that can be followed mechanically.

---

## Skill: Add a Built-in Function

**When:** You need to add a new Maxima function (e.g., `myfunc(x)`).

1. Decide which module: existing (`sets.rs`, `numtheory.rs`, ...) or new
2. If new module:
   ```
   Create crates/eval/src/mymodule.rs
   Add `pub mod mymodule;` to crates/eval/src/lib.rs
   ```
3. Implement:
   ```rust
   pub(crate) fn eval_mymod(name: &str, args: &[Expr]) -> Option<Expr> {
       match name {
           "myfunc" => { /* ... */ Some(result) }
           _ => None,
       }
   }
   ```
4. Wire in `eval.rs` (`eval_funcall` match):
   ```rust
   "myfunc" => {
       if let Some(r) = crate::mymodule::eval_mymod(&func_name, &evaled_args) {
           return r;
       }
       Expr::call(&func_name, evaled_args)
   }
   ```
5. Add test in `crates/eval/tests/mymodule_test.rs`
6. Add to `BUILTIN_FUNCTIONS` in `crates/repl/src/main.rs`
7. Run `cargo test` + walkthrough smoke test

---

## Skill: Add a New Operator / Syntax

**When:** You need new syntax like `{...}` for sets.

1. Add variant to `Operator` enum in `crates/core/src/operator.rs`
2. Add `Display` for it (how it prints)
3. Add constructor on `Expr` (e.g., `Expr::set(items)`)
4. Add token(s) in `crates/parser/src/token.rs`
5. Add lexer rule in `crates/parser/src/lexer.rs`
6. Add parser rule in `crates/parser/src/parser.rs` (`parse_primary`)
7. Add eval rule in `crates/eval/src/eval.rs` (`eval_list`)
8. Test parsing: `cargo run -- -e "{1,2,3};"`

---

## Skill: Fix a Simplification Bug

**When:** An expression doesn't simplify (e.g., `sqrt(4)` stays as `sqrt(4)`).

1. Identify WHERE simplification should happen:
   - **Evaluator** (`eval_math_func`): for function-call-level reduction
   - **Simplifier** (`simplify` in `simp.rs`): for algebraic rules
   - **Integrator** (`normalize_sqrt_powers`): for integration-specific normalization
2. Add the rule in the correct place
3. Check ALL THREE layers — a rule in one may need companions in others
4. Verify numerically: `cargo run -- -e "float(your_expr);"`
5. Run full test suite — simplification changes break many tests

---

## Skill: Fix a Wrong Formula

**When:** An integration/limit/ODE formula produces wrong results.

1. **Verify numerically** first:
   ```python
   # Python check: compare F(1)-F(0) against numerical integration
   from scipy.integrate import quad
   result, _ = quad(lambda x: f(x), 0, 1)
   ```
2. If wrong: **remove the formula entirely** (noun form > wrong answer)
3. Derive the correct formula from scratch (not from the code)
4. Verify the new formula numerically at 3+ test points
5. Check the derivative: `d/dx[F(x)]` should equal `f(x)`
6. Add a numerical verification test

---

## Skill: Refactor a Large File

**When:** A file exceeds ~5K lines and needs splitting.

1. Identify logical sections (grep for `^fn ` to map the structure)
2. Choose a section to extract (integration, matrix, etc.)
3. Create new file, copy functions verbatim (no logic changes)
4. Make functions called from outside `pub(crate)`
5. Add `pub mod newmodule;` to `lib.rs`
6. In the original file, replace functions with `use crate::newmodule::*;`
7. Build and fix visibility/import errors
8. Run full test suite — refactoring MUST be behavior-preserving

---

## Skill: Debug a Test Failure After Display Change

**When:** You changed `Display` for `Expr` and many tests broke.

1. Find all affected assertions:
   ```sh
   grep -rn 'old_pattern' crates/eval/tests/ crates/eval/src/eval.rs
   ```
2. Check the new output: `cargo run -- -e "the_expression;"`
3. Update assertions to match new format
4. For flexible assertions, use `contains`:
   ```rust
   assert!(r.contains("log") && !r.contains("integrate"), "got: {}", r);
   ```
5. Run walkthrough smoke test after all fixes

---

## Skill: Add a Walkthrough

**When:** A new feature needs a tutorial.

1. Create `walkthrough/NN_topic.mac` (next available number)
2. Structure: header comment, sections with `/* --- Section --- */`
3. Each example should be one expression ending with `;`
4. Avoid `if/then/else` and `for` in batch mode (re-parse bug for complex structures)
5. Verify: `cargo run -- -b walkthrough/NN_topic.mac`
6. Update `walkthrough/README.md` table

---

## Skill: Code Review (for LLM reviewers)

**Focus on correctness bugs only. Ignore style.**

1. **Formulas**: Is the math right? Verify at 2 test points.
2. **Overflow**: Are `i64` multiplications checked? Look for `* n` with large `n`.
3. **Match order**: Are specific arms before general ones?
4. **Panics**: Any `unwrap()` on user input? Index without bounds check?
5. **Infinite recursion**: Does `simplify` call `meval` or vice versa?
6. **Display round-trip**: Would `Display → parse` reproduce the original?
7. **Borrow checker**: Is `env` borrowed immutably while also mutably?

---

## Skill: Sprint Execution

**Template for a new feature sprint:**

```
1. Plan
   - What: [function name and behavior]
   - Where: [module name]
   - Test: [expected input → output pairs]

2. Implement
   - Write module function
   - Wire into eval.rs
   - Register in lib.rs

3. Test
   - cargo build (zero errors)
   - cargo test (zero failures)
   - Manual: cargo run -- -e "test_expr;"
   - Walkthrough smoke test

4. Document
   - Add walkthrough if user-facing
   - Update tab completion
   - Update README/user-manual if significant

5. Commit
   - Single commit with descriptive message
   - Include: what was added, test count, zero failures
```

---

## Skill: Investigate a "Returns Noun Form" Bug

**When:** A function returns its unevaluated form instead of computing.

1. Check if the function is wired in `eval_funcall` match arms
2. Check if the function name matches exactly (case-sensitive)
3. Check if input types match (e.g., `to_i64` fails on `Rational`)
4. Add `eprintln!("DEBUG: ...")` temporarily to trace dispatch
5. Common causes:
   - Function not in the match arms at all
   - `expr_to_poly` fails on symbolic coefficients
   - `to_f64` fails on symbolic expressions
   - Pattern match misses because of different `Expr` structure

---

## Skill: Performance Tuning

**When:** An operation is too slow.

1. Profile: `cargo run -- -e "slow_expr;"` with `time`
2. Common hot paths:
   - `simplify()` called too often (cache results with `simplified: true`)
   - Polynomial GCD on large polynomials (check `poly_gcd` degree)
   - `expand()` on high-degree expressions (exponential blowup)
3. Do NOT parallelize — the `Environment` is single-threaded by design
4. Prefer `i64` over `BigInt` for typical-size numbers
