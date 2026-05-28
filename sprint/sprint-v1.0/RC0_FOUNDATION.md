# RC0 — Foundation

**Goal:** Rust project skeleton with expression types, basic arithmetic,
and a minimal REPL that can evaluate `1+1` → `2`.

**Exit criteria:** `cargo test` passes, REPL handles integer arithmetic.

---

## Sprint 0.1 — Project Skeleton

**Duration:** 1 week

### Tasks

- [ ] Initialize `maxima-kernel/` as a Cargo workspace
  - `maxima-kernel/Cargo.toml` (workspace root)
  - `maxima-kernel/crates/core/` — expression types, traits
  - `maxima-kernel/crates/parser/` — tokenizer + parser
  - `maxima-kernel/crates/eval/` — evaluator
  - `maxima-kernel/crates/repl/` — REPL binary
- [ ] Set up CI (GitHub Actions): `cargo build`, `cargo test`, `cargo clippy`
- [ ] Add `ARCHITECTURE.md` in `maxima-kernel/` describing crate layout
- [ ] Choose and pin Rust edition (2024) and MSRV

### Verification

```
cargo build          # compiles
cargo test           # 0 tests, 0 failures
cargo clippy         # no warnings
```

---

## Sprint 0.2 — Expression Representation

**Duration:** 2 weeks

### Tasks

- [ ] Define `Expr` enum in `core`:
  ```rust
  pub enum Expr {
      Integer(i64),
      BigInt(Box<num::BigInt>),
      Float(f64),
      BigFloat { mantissa: BigInt, exponent: i64, precision: u64 },
      Symbol(SymbolId),
      String(Box<str>),
      List { operator: Operator, args: Vec<Expr> },
  }
  ```
- [ ] Implement symbol interning table (`SymbolId` → string mapping)
- [ ] Define `Operator` enum covering core operators:
  `MPlus, MTimes, MExpt, MEqual, MList, MMatrix, MLambda, ...`
- [ ] Implement `Display` for `Expr` (1D flat output)
- [ ] Implement `PartialEq` for structural equality
- [ ] Add serialization round-trip tests (Expr → string → Expr)
- [ ] Implement expression builder helpers:
  `Expr::add(a, b)`, `Expr::mul(a, b)`, `Expr::pow(a, b)`
- [ ] Add simplification flag on `List` variant (mirrors Lisp `simp` tag)

### Tests

```
#[test] fn integer_display() { assert_eq!(Expr::Integer(42).to_string(), "42"); }
#[test] fn symbol_intern()   { let id = intern("x"); assert_eq!(resolve(id), "x"); }
#[test] fn add_display()     { assert_eq!(Expr::add(int(1), int(2)).to_string(), "1+2"); }
#[test] fn nested_expr()     { /* (x+1)^2 round-trip */ }
```

---

## Sprint 0.3 — Integer Arithmetic + Minimal REPL

**Duration:** 2 weeks

### Tasks

- [ ] Implement integer arithmetic in `eval`:
  - Addition, subtraction, multiplication, integer division, exponentiation
  - Overflow from `i64` → `BigInt` promotion
- [ ] Implement minimal tokenizer (integers, `+`, `-`, `*`, `/`, `^`, `(`, `)`, `;`)
- [ ] Implement Pratt parser for arithmetic expressions with correct precedence
- [ ] Build REPL loop in `repl` crate:
  - Read line → tokenize → parse → eval → print
  - Prompt: `(%i1)` / `(%o1)` labels (matching Maxima convention)
  - Handle `quit;` to exit
- [ ] Error handling: parse errors and arithmetic errors display messages, don't crash

### Tests

```
#[test] fn eval_add()       { assert_eval("1+1;", "2"); }
#[test] fn eval_mul()       { assert_eval("3*4;", "12"); }
#[test] fn eval_precedence(){ assert_eval("2+3*4;", "14"); }
#[test] fn eval_parens()    { assert_eval("(2+3)*4;", "20"); }
#[test] fn eval_power()     { assert_eval("2^10;", "1024"); }
#[test] fn eval_bigint()    { assert_eval("2^100;", "1267650600228229401496703205376"); }
#[test] fn eval_negative()  { assert_eval("-3+5;", "2"); }
#[test] fn eval_nested()    { assert_eval("((1+2)^2-1)*3;", "24"); }
```

---

## Deliverable

A `maxima-kernel` binary that:
```
$ maxima-kernel
Maxima Kernel (Rust) v0.1.0
(%i1) 1+1;
(%o1)                          2
(%i2) 2^100;
(%o2)            1267650600228229401496703205376
(%i3) quit;
```
