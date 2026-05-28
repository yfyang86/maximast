# RC1 — Parser + Evaluator

**Goal:** Parse and evaluate core Maxima syntax: variables, function
definitions, conditionals, loops, lists, arrays. Pass `rtest1.mac`.

**Exit criteria:** `cargo test` passes all unit tests;
`rtest1.mac` expected/actual pairs pass through the Rust kernel.

---

## Sprint 1.1 — Full Tokenizer

**Duration:** 2 weeks

### Tasks

- [ ] Extend tokenizer to handle all Maxima tokens:
  - Identifiers: `foo`, `%pi`, `%e`, `%i`
  - Numbers: integers, floats (`3.14`), scientific (`1.5e-3`), bigfloats (`1.0b0`)
  - Strings: `"hello world"` with escape sequences
  - Operators: `:`, `:=`, `::`, `::=`, `=`, `#`, `<`, `>`, `<=`, `>=`
  - Delimiters: `(`, `)`, `[`, `]`, `,`, `$`, `;`
  - Logical: `and`, `or`, `not`
  - Quote: `'expr`, `''expr`
  - Comments: `/* ... */` (nestable)
- [ ] Track source positions (line, column) for error messages
- [ ] Property-test: roundtrip arbitrary tokens

### Tests

```
#[test] fn lex_float()     { tokens("3.14")   == [Float(3.14)] }
#[test] fn lex_string()    { tokens("\"hi\"") == [Str("hi")] }
#[test] fn lex_assign()    { tokens("x:1")    == [Ident("x"), Colon, Int(1)] }
#[test] fn lex_funcdef()   { tokens("f(x):=x^2") == [...] }
#[test] fn lex_comment()   { tokens("1+/* c */2") == [Int(1), Plus, Int(2)] }
```

---

## Sprint 1.2 — Full Parser

**Duration:** 3 weeks

### Tasks

- [ ] Parse Maxima statements:
  - Assignment: `x : expr`
  - Function definition: `f(x) := expr`
  - Conditional: `if ... then ... elseif ... else ...`
  - Loops: `for i:1 thru n do ...`, `while ... do ...`
  - Block: `block([locals], stmt1, stmt2, ...)`
  - Lambda: `lambda([x, y], body)`
  - List: `[a, b, c]`
  - Matrix: `matrix([1,2],[3,4])`
  - Array access: `a[i]`, `a[i,j]`
  - Subscripted functions: `t[n](x) := ...`
  - Sequencing: `(stmt1, stmt2, ...)` with `,` operator
  - Quote: `'f(x)` (suppress evaluation)
  - Double-quote: `''expr` (force evaluation during parse)
- [ ] Operator precedence matching Maxima exactly (reference: `nparse.lisp` tables)
- [ ] Meaningful parse error messages with location

### Tests

```
#[test] fn parse_assign()     { parse("x:1;") == Assign(sym("x"), int(1)) }
#[test] fn parse_funcdef()    { parse("f(x):=x^2;") == FuncDef("f", ["x"], pow(sym("x"), int(2))) }
#[test] fn parse_if()         { parse("if x>0 then 1 else -1;") == ... }
#[test] fn parse_for()        { parse("for i:1 thru 5 do print(i);") == ... }
#[test] fn parse_block()      { parse("block([x:1], x+1);") == ... }
#[test] fn parse_list()       { parse("[1,2,3];") == list([int(1), int(2), int(3)]) }
#[test] fn parse_matrix()     { parse("matrix([1,2],[3,4]);") == ... }
#[test] fn parse_nested_sub() { parse("t[n](x):=2*x;") == ... }
```

---

## Sprint 1.3 — Core Evaluator

**Duration:** 3 weeks

### Tasks

- [ ] Implement `meval` dispatch:
  - Atoms: symbol lookup (global bindings), number/string passthrough
  - Function call: evaluate args, look up definition, apply
  - Special forms: `:=` (define), `:` (assign), `if`, `for`, `while`,
    `do`, `block`, `lambda`, `return`, `go` (for block labels)
- [ ] Environment model:
  - Global symbol table (bindings, function definitions, properties)
  - Local scope via `block([vars], ...)` and function args
  - Dynamic scoping (matching Maxima semantics, not lexical)
- [ ] Built-in functions (first batch):
  - `print`, `display`
  - `is`, `equal`
  - `first`, `rest`, `last`, `length`, `append`, `cons`
  - `map`, `apply`
  - `atom`, `numberp`, `integerp`, `floatnump`, `listp`
  - `error`, `catch`, `throw`
- [ ] Labels: `%i1`, `%o1`, ... stored and retrievable via `%` and `%%`
- [ ] `ev(expr, bindings...)` — core evaluation-with-context function
- [ ] `kill(...)` — clear definitions

### Tests

```
#[test] fn eval_assign()      { run("x:5; x+1;") == "6" }
#[test] fn eval_funcdef()     { run("f(x):=x^2; f(3);") == "9" }
#[test] fn eval_if_true()     { run("if 1>0 then 42 else 0;") == "42" }
#[test] fn eval_for_sum()     { run("s:0; for i:1 thru 5 do s:s+i; s;") == "15" }
#[test] fn eval_block()       { run("block([x:3], x^2);") == "9" }
#[test] fn eval_lambda()      { run("f:lambda([x],x+1); f(5);") == "6" }
#[test] fn eval_list_ops()    { run("first([a,b,c]);") == "a" }
#[test] fn eval_ev()          { run("f(x):=x^2+y; ev(f(2), y:7);") == "11" }
#[test] fn eval_kill()        { run("x:5; kill(x); x;") == "x" }
#[test] fn eval_dynamic_scope() { run("a:1; f():=a; g():=block([a:2], f()); g();") == "2" }
```

---

## Sprint 1.4 — rtest1.mac Compatibility

**Duration:** 2 weeks

### Tasks

- [ ] Build test harness that reads `rtest*.mac` format:
  - Pairs of lines: `input; expected_output$`
  - Compare evaluated input against expected output
  - Report pass/fail per pair with line numbers
- [ ] Run `rtest1.mac` through Rust kernel, fix failures iteratively
- [ ] Handle Maxima-specific constructs appearing in rtest1:
  - `functions` (list defined functions)
  - `values` (list assigned variables)
  - `arrays` (list defined arrays)
  - Subscripted function definitions: `t[n](x) := ...`
  - `kill(functions, values, arrays)`
  - `ratexpand`, `ev` with substitution
  - `sum(expr, var, lo, hi)`
- [ ] Track pass rate: target 100% of rtest1.mac

### Tests

```
#[test] fn rtest1_full() { assert_rtest_passes("tests/rtest1.mac"); }
```

---

## Deliverable

```
$ maxima-kernel
(%i1) f(x) := x^2 + y;
(%o1)                       f(x):=x^2+y
(%i2) ev(f(2), y:7);
(%o2)                          11
(%i3) for i:1 thru 5 do print(i);
1 2 3 4 5
(%o3)                         done
(%i4) block([s:0], for i:1 thru 10 do s:s+i, s);
(%o4)                          55
```
