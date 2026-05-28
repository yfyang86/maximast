# Maxima Kernel (Rust) — Technical Report

**Version 5.0** | May 2026

A ground-up reimplementation of the Maxima computer algebra system kernel
in Rust, targeting correctness, performance, and extensibility.

---

## 1. Architecture

### 1.1 Design Goals

The original Maxima (1968–present) is written in Common Lisp and carries
decades of accumulated complexity. This reimplementation pursues:

- **Correctness first**: exact rational arithmetic, no silent precision loss
- **Modularity**: separate crates for parsing, evaluation, polynomials
- **Extensibility**: plugin API for Rust-native function registration
- **Compatibility**: Maxima syntax and semantics where practical

### 1.2 Crate Structure

```
maxima-kernel/           22,361 lines of Rust
├── crates/core/           832 lines — Expression types, symbol interning
├── crates/parser/       1,623 lines — Tokenizer + Pratt parser
├── crates/poly/         3,653 lines — Polynomial arithmetic, GCD, factoring
├── crates/eval/        15,763 lines — Evaluator, simplifier, all math
└── crates/repl/           490 lines — Interactive REPL with tab completion
```

### 1.3 Core Data Types

**Expr** — the universal expression type:

```rust
pub enum Expr {
    Integer(i64),                        // exact integer
    BigInt(Box<BigInt>),                 // arbitrary-precision integer
    Rational { num: i64, den: i64 },     // exact rational p/q
    Float(f64),                          // IEEE 754 double
    Symbol(SymbolId),                    // interned symbol (e.g., x, %pi)
    String(Box<str>),                    // string literal
    List { op: Operator, args: Vec<Expr>, simplified: bool },
}
```

All mathematical expressions are `List` nodes with an `Operator` tag.
For example, `x^2 + 1` is:

```
MPlus([MExpt([Symbol(x), Integer(2)]), Integer(1)])
```

**Operator** — 28 variants covering arithmetic (`MPlus`, `MTimes`, `MExpt`),
comparison, logic, control flow, and a `Named(SymbolId)` catch-all for
function calls.

**SymbolId** — a `u32` index into a global interned string table.
`intern("sin")` returns the same `SymbolId` across the program lifetime,
enabling O(1) symbol comparison.

### 1.4 Evaluation Pipeline

```
Input string
  → Lexer (token stream)
  → Pratt Parser (AST: Vec<Expr>)
  → Evaluator (meval recursive dispatch)
  → Simplifier (canonical form)
  → Display (formatted output)
```

The evaluator (`meval`) is a recursive match on the expression tree:

1. **Atoms** (Integer, Float, Symbol) — look up variable bindings or return self
2. **Arithmetic** (MPlus, MTimes, MExpt) — evaluate args, simplify
3. **Named functions** — 200+ match arms dispatching to specialized modules
4. **User-defined functions** — dynamic lookup in `Environment.functions`
5. **Native plugins** — highest-priority lookup in `Environment.native_functions`

### 1.5 Module Organization

The evaluator delegates to domain-specific modules:

| Module | Lines | Responsibility |
|--------|-------|----------------|
| `eval.rs` | 6,973 | Core dispatch, arithmetic, control flow, lists |
| `integrate.rs` | 3,091 | Indefinite/definite integration engine |
| `simp.rs` | 874 | Algebraic simplification (canonical forms) |
| `gruntz.rs` | 823 | Limit computation (Gruntz MRV algorithm) |
| `sets.rs` | 165 | Set algebra ({}-syntax, union, intersection) |
| `strings.rs` | 116 | String manipulation (slength, split, ...) |
| `numtheory.rs` | 215 | Number theory (ifactors, CRT, fibonacci) |
| `expr_manip.rs` | 305 | Expression manipulation (multthru, at, ...) |
| `laplace.rs` | 298 | Laplace/inverse Laplace transforms |
| `ode.rs` | 287 | ODE solver (ode2, separable, linear, const-coeff) |
| `poly_analysis.rs` | 90 | Resultant, discriminant, content |
| `log_trig.rs` | 111 | logcontract, logexpand |
| `env.rs` | 378 | Environment (scopes, functions, plugin registry) |
| `helpers.rs` | 397 | Utility functions (subst, contains_var, ...) |

### 1.6 Plugin API

Native Rust functions can be registered at runtime:

```rust
pub type NativeFn = fn(&[Expr], &mut Environment) -> Expr;

env.register_native("my_func", my_func, 1, Some(3));
```

Dispatch priority: native → user-defined → lambda → autoload → noun form.
Native functions survive `kill(all)`, behaving like built-ins.

### 1.7 Package System

- `load("file.mac")` — evaluate a Maxima script
- `require("file.mac")` — load-once semantics
- `setup_autoload("file.mac", f1, f2)` — lazy loading on first call
- Configurable `search_paths`, nested load support
- `loaded_files()`, `load_pathname()` for introspection

---

## 2. Features

### 2.1 Arithmetic

| Feature | Details |
|---------|---------|
| Exact integers | i64 with BigInt overflow |
| Exact rationals | p/q with automatic GCD reduction |
| IEEE 754 float | `float()` conversion |
| Number theory | primep, gcd, mod, binomial, factorial |
| Extended NT | ifactors, totient, divisors, next_prime, fibonacci, CRT |
| Rounding | floor, ceiling, truncate, round (banker's) |

### 2.2 Algebra

| Feature | Functions |
|---------|-----------|
| Expansion | expand, ratexpand |
| Factoring | factor (integer coefficients, algebraic over Q(√d)) |
| Simplification | ratsimp, radcan, trigsimp, trigexpand, trigreduce |
| Partial fractions | partfrac |
| GCD | gcd (integer and polynomial) |
| Substitution | subst, ratsubst, at |
| Polynomial analysis | coeff, hipow, lopow, content, primpart, resultant, discriminant |
| Expression manip | multthru, xthru, collectterms |
| Log rules | logcontract, logexpand |

### 2.3 Calculus

| Feature | Functions |
|---------|-----------|
| Differentiation | diff (arbitrary order, chain rule) |
| Indefinite integration | integrate (table, Hermite, partfrac, substitution, algebraic) |
| Definite integration | integrate with limits (Gamma, Gaussian, Laplace, residues) |
| Limits | limit (L'Hopital, Gruntz MRV, directional, abs-aware) |
| Taylor series | taylor |
| Summation | sum (Gosper, telescoping, closed-form) |
| Products | product |

### 2.4 Transforms

| Feature | Functions |
|---------|-----------|
| Laplace | laplace(f, t, s) with linearity, shift theorem |
| Inverse Laplace | ilt(F, s, t) with table-driven matching |

### 2.5 Solving

| Feature | Functions |
|---------|-----------|
| Polynomial | solve (linear, quadratic formula, higher-degree roots) |
| Linear systems | linsolve |
| ODEs | ode2 (separable, linear first-order, const-coeff second-order) |
| Initial conditions | ic1 |

### 2.6 Linear Algebra

| Feature | Functions |
|---------|-----------|
| Construction | matrix, ident, zeromatrix |
| Access | M[i,j], M[i] (row), transpose |
| Decomposition | determinant, invert, rank |
| Eigenvalues | eigenvalues, eigenvectors, charpoly |
| Power | M^^n (repeated squaring) |

### 2.7 Data Structures

| Type | Functions |
|------|-----------|
| Lists `[...]` | first, second, ..., fifth, last, rest, cons, endcons, append, reverse, sort, length, map, makelist, member, delete, sublist, flatten |
| Sets `{...}` | union, intersection, setdifference, symdifference, elementp, subsetp, disjointp, cardinality, powerset, setify, listify |
| Strings `"..."` | sconcat, slength, charat, substring, ssearch, ssubst, strim, split, supcase, sdowncase, sequal, parse_string |

### 2.8 System

| Feature | Functions |
|---------|-----------|
| Assumptions | assume, forget, is, facts |
| Boolean logic | and, or, not (De Morgan, absorption) |
| Control flow | if/then/else/elseif, for/while/do, block, return |
| Output | print, display, tex (LaTeX) |
| File I/O | load, require, save, stringout, printfile |
| Introspection | functions, values, properties, op, args, nterms |

---

## 3. Algorithms

### 3.1 Polynomial Arithmetic

The `poly` crate implements sparse univariate polynomials over Q:

```rust
pub struct Poly {
    pub var: SymbolId,
    pub terms: Vec<(u32, Coeff)>,  // (exponent, coefficient) pairs
}
```

**GCD**: subresultant PRS (Pseudo-Remainder Sequence) algorithm, avoiding
coefficient explosion that plagues naive Euclidean GCD over Z[x].

**Factoring**: trial division for small factors, then Berlekamp or
algebraic factoring over Q(√d) via the Trager norm-shift method.

**Square-free decomposition**: `sqfr(p)` via `gcd(p, p')`.

### 3.2 Integration Engine

The integration engine uses a cascade of methods:

```
1. Risch-Norman heuristic (fast path for transcendental integrands)
2. Table lookup (200+ known integrals)
3. Linearity: ∫(af+bg) = a∫f + b∫g
4. Constant extraction: ∫c·f = c·∫f
5. Power rule: ∫x^n = x^(n+1)/(n+1)
6. Hermite reduction (rational functions → log + atan decomposition)
7. Partial fraction decomposition → elementary integrals
8. Substitution detection (u-substitution with automatic u'/u matching)
9. Integration by parts (automatic selection of u and dv)
10. Algebraic factoring over Q(√d) for irreducible quartics
11. sqrt normalization: convert sqrt(x) to x^(1/2) for power rule
```

**Hermite reduction**: For `∫ P(x)/Q(x) dx` where `Q` has repeated factors,
decomposes into `rational_part + ∫ log_part` using extended GCD:

```
P/Q = d/dx(A/D) + B/E
```

where `D = gcd(Q, Q')`, `E = Q/D`, and A, B are found by solving
`s·Qi + t·Qi' = A` via extended Euclidean algorithm.

**Definite integrals**: Residue theorem for `∫_{-∞}^{∞} P(x)/Q(x) dx`
via upper half-plane poles, plus Gamma function table for `∫_0^∞ x^n·e^{-x} dx`.

### 3.3 Limit Computation

Two-tier approach:

1. **Direct substitution + L'Hopital**: For finite limits, substitute the
   point. If 0/0 results, iterate differentiation up to 5 times.

2. **Gruntz MRV algorithm**: For limits at infinity, compute the set of
   Most Rapidly Varying subexpressions, substitute the dominant term,
   and recurse. Handles nested exp/log towers like
   `limit(exp(x + exp(-x)) - exp(x), x, inf) = 1`.

**Abs-aware limits**: When `abs(f(x))` appears, resolve by computing
the derivative sign of `f` at the limit point to determine which branch
to take. Bidirectional limits compare both sides and return `und` if they disagree.

### 3.4 Summation

**Gosper algorithm**: For `∑_{k=lo}^{hi} f(k)`, finds a closed form
`g(k)` such that `g(k+1) - g(k) = f(k)`, then evaluates `g(hi+1) - g(lo)`.

**Telescoping detection**: Identifies sums like `∑ 1/(k(k+1))` that
telescope to `1 - 1/(n+1)`.

**Polynomial sums**: Closed forms for `∑ k^p` via Faulhaber's formulas.

### 3.5 Laplace Transforms

Table-driven with structural decomposition:

- **Linearity**: `L{af + bg} = aL{f} + bL{g}`
- **Shift theorem**: `L{e^{at}f(t)} = F(s-a)`
- **Table entries**: 1, t^n (→ n!/s^{n+1}), exp(at), sin(wt), cos(wt),
  sinh(wt), cosh(wt)

Inverse transform uses the same table in reverse with pattern matching.

### 3.6 ODE Solver

**First order**:
- Separable: `dy/dx = f(x)·g(y)` → `∫ dy/g(y) = ∫ f(x) dx + C`
- Linear: `dy/dx + P(x)y = Q(x)` → integrating factor `μ = e^{∫P dx}`

**Second order** (constant coefficients `ay'' + by' + cy = 0`):
- Characteristic equation `ar² + br + c = 0`
- Distinct real roots → `y = k₁e^{r₁x} + k₂e^{r₂x}`
- Complex roots α±βi → `y = e^{αx}(k₁cos(βx) + k₂sin(βx))`
- Repeated root r → `y = (k₁ + k₂x)e^{rx}`

### 3.7 Algebraic Number Fields

The `poly_alg` module implements polynomials over Q(α):

```rust
pub struct AlgNumber {
    pub coeffs: Vec<(i64, i64)>,  // [(num, den), ...] in basis 1, α, α², ...
}
```

Used for factoring polynomials like `x⁴+1` which is irreducible over Q
but factors as `(x²+√2·x+1)(x²-√2·x+1)` over Q(√2).

**Trager norm method**: To factor `f(x)` over Q(α), compute
`N(x) = Norm_{Q(α)/Q}(f(x - c·α))` for trial values of `c`,
factor N(x) over Q, then recover factors via `gcd(f, g(x+c·α))`.

### 3.8 Simplification

The simplifier maintains canonical forms:

- **Flatten**: `(a+b)+c → a+b+c` (associativity)
- **Collect**: `2x+3x → 5x` (like-term combination via coefficient maps)
- **Sort**: canonical ordering of terms for deterministic output
- **Distribute**: `-1·(a+b) → -a + -b` (sign distribution)
- **Power**: `(a^b)^c → a^{bc}`, `sqrt(n) → k·sqrt(m)` for perfect-square extraction
- **Identity**: `x^0 → 1`, `x^1 → x`, `x·1 → x`, `x+0 → x`

---

## 4. Walkthroughs

21 interactive tutorials in `walkthrough/`, covering the full feature set:

| # | Topic | Key concepts |
|---|-------|-------------|
| 01 | Arithmetic | Exact rationals, big integers, float conversion |
| 02 | Algebra | expand, factor, ratsimp, partfrac, trig identities |
| 03 | Calculus | diff, integrate, definite integrals, Taylor series |
| 04 | Solving | Polynomial roots, quadratic formula, linear systems |
| 05 | Limits | L'Hopital, infinity, indeterminate forms |
| 06 | Matrices | determinant, invert, eigenvalues, charpoly |
| 07 | Summation | Gosper closed forms, telescoping, binomials |
| 08 | Advanced Integration | Algebraic integrands, Gaussian, Laplace table |
| 09 | Assumptions | assume/forget, abs simplification, boolean logic |
| 10 | Programming | Functions, lambda, lists, makelist |
| 11 | File I/O | load, require, save, autoload |
| 12 | Plugin API | NativeFn registration (Rust reference) |
| 13 | LaTeX Output | tex() rendering |
| 14 | Matrix Applications | M^^n, Fibonacci via matrix power |
| 15 | Game Solver | 24-game (recursive programming showcase) |
| 16 | Number Theory | floor/ceiling/round, mod, gcd, primep |
| 17 | Sets | {}-syntax, union, intersection, powerset |
| 18 | Strings | slength, split, substring, parse_string |
| 19 | Number Theory (ext) | ifactors, CRT, fibonacci, Jacobi symbol |
| 20 | Laplace Transforms | laplace, ilt, shift theorem |
| 21 | ODEs | ode2 for first and second order equations |

Run any walkthrough:
```sh
maxima-repl -b walkthrough/03_calculus.mac
```

---

## 5. Gaps and Future Work

### 5.1 Known Limitations

| Area | Limitation |
|------|-----------|
| Polynomial factoring | Only integer coefficients; symbolic coefficients return noun form |
| Integration | No Euler substitution for `∫ R(x, √(ax²+bx+c)) dx` |
| Definite integrals | Partial fraction with two irreducible quadratics unsupported |
| ODE solver | No variation of parameters, no non-homogeneous second-order |
| Matrix | No element-wise +/-/* for matrices (only scalar * matrix) |
| Simplification | `exp(0)` not simplified to 1 within `simplify()` (only in evaluator) |
| Batch mode | `Display` round-trip imperfect for `if/for/block` in re-parse path (mostly fixed) |
| Large integers | sqrt and rounding use f64 intermediate — precision loss above 2^53 |

### 5.2 Missing Maxima Features (ordered by impact)

**High impact**:
- `bfloat` (arbitrary-precision floating point)
- Pattern matching (`matchdeclare`, `defrule`, `tellsimp`)
- Plotting (`plot2d`, `plot3d` via gnuplot)
- `residue(f, z, z0)` for complex analysis

**Medium impact**:
- `trigrat` (rational trig form)
- `halfangles` (half-angle substitution)
- Variation of parameters for non-homogeneous ODEs
- `bc2` (boundary conditions for second-order ODEs)
- `nroots` / `realroots` (Sturm chain root isolation)
- Matrix arithmetic (element-wise operations)

**Lower impact**:
- `catch`/`throw` (structured error handling)
- Tensor algebra (`itensor`/`ctensor`)
- `draw` package (2D/3D graphics)
- `contrib_ode` (advanced ODE methods)

### 5.3 Planned Enhancements

| Enhancement | Description |
|-------------|-------------|
| Dynamic plugins | `.so`/`.dylib` loading via `dlopen` for `NativeFn` |
| Hash-consed DAG | O(1) structural equality via arena allocation |
| Criterion benchmarks | Proper microbenchmark suite for hot paths |
| WASM target | Compile to WebAssembly for browser-based CAS |

---

## 6. Development Hints

### 6.1 Building and Testing

```sh
cd maxima-kernel
cargo build                    # debug build
cargo build --release          # optimized build
cargo test                     # run all 920 tests
cargo test --test benchmark    # run performance benchmarks
cargo run --bin maxima-repl    # start REPL
```

### 6.2 Adding a New Built-in Function

1. Choose the appropriate module (or create a new one in `crates/eval/src/`)
2. Implement the function:
   ```rust
   pub(crate) fn eval_myfunc(name: &str, args: &[Expr]) -> Option<Expr> {
       match name {
           "myfunc" => { /* implementation */ Some(result) }
           _ => None,
       }
   }
   ```
3. Register in `lib.rs`: `pub mod mymodule;`
4. Wire in `eval.rs` eval_funcall match:
   ```rust
   "myfunc" => {
       if let Some(r) = crate::mymodule::eval_myfunc(&func_name, &evaled_args) {
           return r;
       }
       Expr::call(&func_name, evaled_args)
   }
   ```
5. Add tests in `crates/eval/tests/mymodule_test.rs`
6. Add to tab completion in `crates/repl/src/main.rs` `BUILTIN_FUNCTIONS`

### 6.3 Adding a New Operator

1. Add variant to `Operator` enum in `crates/core/src/operator.rs`
2. Add `Display` implementation for output
3. Add constructor on `Expr` (e.g., `Expr::set()`)
4. Add token(s) in `crates/parser/src/token.rs`
5. Add lexing rule in `crates/parser/src/lexer.rs`
6. Add parse rule in `crates/parser/src/parser.rs`
7. Add evaluation rule in `crates/eval/src/eval.rs` `eval_list`

### 6.4 Writing a Rust Plugin

```rust
use maxima_eval::{Environment, NativeFn};
use maxima_core::Expr;

fn my_function(args: &[Expr], _env: &mut Environment) -> Expr {
    // Access args[0], args[1], etc.
    // Return an Expr
    match &args[0] {
        Expr::Integer(n) => Expr::int(n * 2),
        _ => Expr::call("my_function", args.to_vec()),
    }
}

// Register:
env.register_native("my_function", my_function as NativeFn, 1, Some(1));
```

### 6.5 Key Design Decisions

**Why not reuse the Lisp code?** The original Maxima's Lisp codebase
(~300K lines) is tightly coupled to Common Lisp's runtime (CLOS, conditions,
dynamic variables). A Rust port gains memory safety, zero-cost abstractions,
and easy cross-compilation, but requires reimplementing algorithms.

**Why `i64` for rationals instead of BigInt?** Performance. Most CAS
operations involve small numbers. `i64` arithmetic is single-instruction
on modern CPUs. BigInt is used only for `n!` and `2^n` results that
overflow i64, with i128 intermediates for rational reduction.

**Why separate `simplify()` from `meval()`?** The evaluator handles
function dispatch and variable lookup. The simplifier handles algebraic
canonicalization (like-term collection, power rules). Keeping them separate
avoids infinite loops where simplification triggers evaluation which
triggers simplification.

**Why `Named(SymbolId)` instead of enum variants for functions?**
Extensibility. With ~250 built-in functions and user-defined functions,
a closed enum would be impractical. `Named` allows open-ended dispatch
via string matching, with the interning system making comparison O(1).

### 6.6 Code Conventions

- Functions return `Option<Expr>` when they might not handle an input
  (falls through to noun form)
- `simplify()` is idempotent — calling it twice produces the same result
- `meval()` always fully evaluates — no lazy/partial evaluation
- Test names follow `modulename_feature` pattern
- Integration tests live in `crates/eval/tests/` as separate `.rs` files
- Walkthroughs are self-contained `.mac` scripts runnable in batch mode

### 6.7 Performance Notes

Typical operations on a modern CPU (single-threaded):

| Operation | Time |
|-----------|------|
| `expand((x+1)^50)` | ~1ms |
| `factor(x^12-1)` | ~0.5ms |
| `integrate(1/(x^4+1), x)` | ~2ms |
| `sum(k^2, k, 1, 5000)` | ~5ms |
| `determinant(5×5 symbolic)` | ~10ms |
| `920 tests` | ~0.2s |

No parallelism is used. The evaluator is single-threaded by design
(mutable `Environment` prevents data races at the type level).

---

## Appendix A: Version History

| Version | Tests | Key Milestone |
|---------|-------|---------------|
| v1.0 | 326 | Parser, evaluator, simplifier |
| v2.0 | 777 | Hermite reduction, Risch tower, Gruntz MRV, series |
| v3.0 | 777 | Algebraic fields, classic Gruntz, LRT, Zeilberger |
| v4.0 | 794 | CLI, PolyAlg, algebraic integration, benchmarks |
| v4.0+ | 822 | Plugin API, package system, bug fixes |
| **v5.0** | **920** | **Standard library: sets, strings, numtheory, Laplace, ODE** |

## Appendix B: File Inventory

| Path | Lines | Description |
|------|-------|-------------|
| `crates/core/src/expr.rs` | 684 | Expr enum, Display, constructors |
| `crates/core/src/operator.rs` | 67 | Operator enum (28 variants) |
| `crates/core/src/intern.rs` | 58 | Global symbol interning |
| `crates/parser/src/lexer.rs` | 296 | Tokenizer |
| `crates/parser/src/parser.rs` | 978 | Pratt parser |
| `crates/parser/src/token.rs` | 38 | Token enum |
| `crates/poly/src/poly.rs` | 247 | Sparse polynomial type |
| `crates/poly/src/gcd.rs` | 157 | Polynomial GCD (subresultant PRS) |
| `crates/poly/src/factor.rs` | 330 | Polynomial factoring |
| `crates/poly/src/hermite.rs` | 266 | Hermite reduction, resultant |
| `crates/poly/src/alg_field.rs` | 270 | Algebraic number fields Q(α) |
| `crates/poly/src/poly_alg.rs` | 650 | Polynomials over Q(α), Trager method |
| `crates/eval/src/eval.rs` | 6,973 | Core evaluator |
| `crates/eval/src/integrate.rs` | 3,091 | Integration engine |
| `crates/eval/src/simp.rs` | 874 | Algebraic simplifier |
| `crates/eval/src/gruntz.rs` | 823 | Gruntz limit algorithm |
| `crates/eval/src/laplace.rs` | 298 | Laplace transforms |
| `crates/eval/src/ode.rs` | 287 | ODE solver |
| `crates/eval/src/sets.rs` | 165 | Set algebra |
| `crates/eval/src/numtheory.rs` | 215 | Number theory |
| `crates/eval/src/strings.rs` | 116 | String functions |
| `crates/eval/src/expr_manip.rs` | 305 | Expression manipulation |
| `crates/eval/src/env.rs` | 378 | Environment, plugin registry |
| `crates/repl/src/main.rs` | 490 | REPL with tab completion |
| `walkthrough/*.mac` | 21 files | Interactive tutorials |
| `tests/*.rs` | 1,907 | 920 test cases |
