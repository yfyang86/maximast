# Test Strategy

## Layers of Testing

### 1. Unit Tests (per crate, `cargo test`)

Each Rust crate has module-level tests covering individual functions.
These run in CI on every push.

```
maxima-kernel/
  crates/core/src/expr/tests.rs     — expression construction, equality, display
  crates/core/src/intern/tests.rs   — symbol interning
  crates/parser/src/tests.rs        — tokenizer + parser
  crates/eval/src/tests.rs          — evaluator
  crates/simp/src/tests.rs          — simplifier rules
  crates/rat/src/tests.rs           — rational/polynomial arithmetic
  crates/assume/src/tests.rs        — assumption database
  crates/display/src/tests.rs       — 2D rendering, TeX output
  crates/io/src/tests.rs            — file loading, string ops
```

### 2. Integration Tests (`cargo test --test`)

End-to-end tests that feed Maxima input strings through the full pipeline
(parse → eval → simplify → display) and compare output.

```
maxima-kernel/tests/
  integration/
    arithmetic.rs       — basic numeric evaluation
    algebra.rs          — symbolic manipulation
    calculus.rs         — differentiation, basic integration
    linear_algebra.rs   — matrices
    assumptions.rs      — assume/is/asksign
    io.rs               — file loading, string operations
```

Helper macro:
```rust
/// Assert that evaluating `input` produces `expected` output.
macro_rules! assert_maxima {
    ($input:expr, $expected:expr) => {
        let result = eval_to_string($input);
        assert_eq!(result.trim(), $expected, "Input: {}", $input);
    };
}
```

### 3. Compatibility Tests (rtest harness)

A dedicated test binary that reads original Maxima `rtest*.mac` files
and verifies the Rust kernel produces matching results.

```
maxima-kernel/tests/
  compat/
    rtest_runner.rs     — .mac file parser and test executor
    mod.rs              — registers all rtest files
```

**rtest format:**
```
input_expression;
expected_output$
```

The runner:
1. Reads pairs of (input, expected) from the `.mac` file
2. Evaluates `input` through the Rust kernel
3. Compares result against `expected` (structural equality, not string)
4. Reports pass/fail per pair with line numbers

**Handling known failures:**
```rust
/// Tests marked as known-fail are tracked but don't fail CI.
/// They MUST have an associated issue number.
#[allow_fail(issue = "GH-42")]
fn rtest_limit_line_237() { ... }
```

### 4. Property-Based Tests

Using `proptest` or `quickcheck` for invariant testing:

```rust
// Simplification is idempotent
proptest! {
    fn simplify_idempotent(expr in arb_expr()) {
        let s1 = simplify(expr.clone());
        let s2 = simplify(s1.clone());
        assert_eq!(s1, s2);
    }
}

// Parse/display roundtrip
proptest! {
    fn parse_display_roundtrip(expr in arb_expr()) {
        let displayed = display_1d(&expr);
        let reparsed = parse(&displayed);
        assert_eq!(simplify(expr), simplify(reparsed));
    }
}

// Rational arithmetic consistency
proptest! {
    fn rat_add_commutative(a in arb_rational(), b in arb_rational()) {
        assert_eq!(rat_add(&a, &b), rat_add(&b, &a));
    }
}
```

### 5. Benchmark Tests

Using `criterion` for performance regression tracking:

```rust
// Polynomial multiplication scaling
fn bench_poly_mul(c: &mut Criterion) {
    for deg in [10, 50, 100, 500] {
        c.bench_function(&format!("poly_mul_deg_{}", deg), |b| {
            let p = random_poly(deg);
            let q = random_poly(deg);
            b.iter(|| poly_mul(&p, &q));
        });
    }
}

// Factoring performance
fn bench_factor(c: &mut Criterion) {
    let cases = ["x^10-1", "x^20-1", "x^50-1"];
    for case in cases {
        c.bench_function(&format!("factor_{}", case), |b| {
            let expr = parse(case);
            b.iter(|| factor(&expr));
        });
    }
}
```

---

## CI Pipeline

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo test                    # unit + integration
      - cargo test --test compat      # rtest compatibility
      - cargo bench --no-run          # compile benchmarks (don't run in CI)

  compat-report:
    steps:
      - cargo test --test compat -- --report
      # Outputs: "67/99 rtest files passing (68%)"
      # Posts as PR comment if pass rate changed
```

---

## Compatibility Tracking

### rtest Pass Rate by RC

| RC | Target | Files |
|----|--------|-------|
| RC0 | 0/99 | (no rtest support yet) |
| RC1 | 1/99 | rtest1 |
| RC2 | 4/99 | rtest1–4 |
| RC3 | 7/99 | + ask1, boolean, equal |
| RC4 | 11/99 | + rtest5–8 |
| RC5 | 14/99 | + rtest9–11 |
| RC6 | 60+/99 | broad coverage |

### Test result format

Each CI run produces `test-results.json`:
```json
{
  "timestamp": "2026-05-24T12:00:00Z",
  "rc": "RC2",
  "total_files": 99,
  "passing_files": 4,
  "total_pairs": 2847,
  "passing_pairs": 412,
  "files": {
    "rtest1.mac": { "total": 89, "pass": 89, "fail": 0, "status": "PASS" },
    "rtest2.mac": { "total": 73, "pass": 73, "fail": 0, "status": "PASS" },
    "rtest5.mac": { "total": 62, "pass": 41, "fail": 21, "status": "PARTIAL" }
  }
}
```

---

## Comparison Methodology

When comparing Rust kernel output to expected Maxima output:

1. **Structural equality** (preferred): compare expression trees, not strings.
   `x+y` and `y+x` are equal if both simplify to the same canonical form.

2. **Numeric tolerance**: for float results, use relative tolerance of 1e-12.

3. **Canonical form**: both expected and actual are simplified before comparison.
   This handles ordering differences like `a+b` vs `b+a`.

4. **Display normalization**: strip whitespace differences for string comparisons.

5. **Known divergences**: document cases where Rust kernel intentionally
   differs from Lisp Maxima (e.g., different canonical ordering) in
   `KNOWN_DIVERGENCES.md`.
