# Risks & Key Decisions

## Architectural Decisions

### D1: Expression Representation

**Decision:** Enum-based AST with arena allocation.

```rust
pub enum Expr {
    Integer(i64),
    BigInt(Box<num::BigInt>),
    Rational { num: i64, den: i64 },
    BigRational(Box<num::BigRational>),
    Float(f64),
    Symbol(SymbolId),
    String(Box<str>),
    List { op: Operator, simplified: bool, args: Vec<Expr> },
}
```

**Alternatives considered:**
- S-expression lists (like Lisp): too untyped for Rust, loses type safety
- Tagged pointer union: fragile, hard to extend
- Interned graph (like egraph): premature optimization

**Rationale:** Enum gives exhaustive matching, clear memory layout, and natural
Rust idioms. `Vec<Expr>` for args avoids fixed-arity constraints. Arena
allocation can be added later for GC-free performance.

### D2: Scoping Model

**Decision:** Dynamic scoping (matching Maxima semantics).

Maxima uses dynamic scope. The Rust kernel must replicate this exactly,
even though lexical scope is more natural in Rust.

**Implementation:** Environment as a stack of `HashMap<SymbolId, Expr>` frames.
Function calls push a frame, `block` pushes a frame with local bindings.
Symbol lookup walks the stack top-down.

### D3: Crate Boundaries

**Decision:** Fine-grained crates for compilation speed and modularity.

```
core    — Expr, SymbolId, Operator (no dependencies)
parser  — depends on core
eval    — depends on core, parser
simp    — depends on core
rat     — depends on core, simp (polynomial/rational arithmetic)
assume  — depends on core, simp
display — depends on core
io      — depends on core, parser, eval
repl    — depends on all above
```

Circular dependencies are forbidden. If `eval` needs simplification,
it calls through a trait interface, not a direct `simp` crate dependency.

### D4: Error Handling

**Decision:** `Result<Expr, MaximaError>` for recoverable errors; `panic` only
for internal bugs.

```rust
pub enum MaximaError {
    Parse { message: String, line: usize, col: usize },
    Eval { message: String },
    Type { expected: &'static str, got: String },
    Unbound { name: String },
    ArityMismatch { name: String, expected: usize, got: usize },
    UserError { message: String },  // from error("...")
    Interrupt,                       // Ctrl-C
}
```

### D5: BigFloat Strategy

**Decision:** Use `rug` crate (GMP/MPFR bindings) for arbitrary-precision
floats, matching Maxima's bigfloat semantics.

**Alternative:** Pure Rust (`dashu`). Chosen against because MPFR is the
gold standard for correctness and Maxima's bigfloat results are calibrated
against it.

---

## Risks

### R1: Semantic Fidelity (HIGH)

**Risk:** Maxima has 50 years of edge-case behavior baked into Lisp code.
Subtle evaluation-order differences, simplification choices, or scoping
quirks will cause rtest failures that are hard to diagnose.

**Mitigation:**
- rtest compatibility suite as the primary correctness signal
- Read the Lisp source for each feature before implementing
- Document intentional divergences in `KNOWN_DIVERGENCES.md`
- Start with the simplest tests and build up incrementally

### R2: Dynamic Dispatch Overhead (MEDIUM)

**Risk:** Maxima's Lisp code uses property lists extensively for operator
dispatch. A naive Rust translation (HashMap lookups per simplification)
may be slower than expected.

**Mitigation:**
- Use enum dispatch (match) for built-in operators (zero-cost)
- Reserve HashMap dispatch only for user-defined rules (`tellsimp`, `matchdeclare`)
- Benchmark early (Sprint 0.3) to catch issues

### R3: Scope Creep (HIGH)

**Risk:** "Just add limits/integration/special functions to pass more tests"
pulls in 40,000+ lines of specialized math before the kernel is solid.

**Mitigation:**
- Hard boundary: RC0–RC6 covers the kernel only
- Specialized math stays in `.mac` files loaded at runtime
- Stub functions return noun forms (`'integrate(f,x)`) for out-of-scope features
- Future RCs (RC7+) tackle integration, limits, special functions

### R4: Build Complexity (LOW)

**Risk:** Adding a Rust build alongside the existing autotools/ASDF build
creates confusion.

**Mitigation:**
- `maxima-kernel/` is a self-contained Cargo workspace
- Does not interfere with existing `configure`/`make` build
- Can be built and tested independently

### R5: Community Adoption (MEDIUM)

**Risk:** Maxima community may resist a Rust rewrite.

**Mitigation:**
- Keep the Lisp kernel functional and maintained during transition
- Rust kernel is opt-in: users choose which backend to use
- Focus on compatibility (rtest pass rate) as the trust signal
- Maintain identical `.mac` file compatibility

---

## Timeline Estimate

| Phase | Duration | Cumulative |
|-------|----------|------------|
| RC0 — Foundation | 5 weeks | 5 weeks |
| RC1 — Parser + Evaluator | 10 weeks | 15 weeks |
| RC2 — Simplifier | 9 weeks | 24 weeks |
| RC3 — Assumptions | 9 weeks | 33 weeks |
| RC4 — Rational/Polynomial | 10 weeks | 43 weeks |
| RC5 — File Loading | 9 weeks | 52 weeks |
| RC6 — Display + Suite | 12 weeks | 64 weeks |

**Total estimated: ~16 months** for a kernel that passes 60+ rtest files.

This is aggressive. A more conservative estimate with buffer for
debugging and edge cases: **18–20 months**.

---

## Future RCs (Not Planned in Detail)

| RC | Scope | Rough estimate |
|----|-------|---------------|
| RC7 | Symbolic integration (risch, defint) | 4 months |
| RC8 | Limits and series | 3 months |
| RC9 | Special functions (gamma, bessel, elliptic) | 4 months |
| RC10 | Numerical routines (SLATEC port) | 3 months |
| RC11 | Plot subsystem | 2 months |
| RC12 | Full test suite parity (99/99 rtests) | 3 months |
