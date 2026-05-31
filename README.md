# Maxima Kernel (Rust)

A computer algebra system kernel rewritten in Rust, compatible with [Maxima](https://maxima.sourceforge.io/) syntax.

## Quick Start

```sh
cd maxima-kernel
cargo run                  # start REPL
cargo run -- -e "integrate(1/(x^4+1), x);"   # evaluate expression
cargo run -- -b walkthrough/03_calculus.mac    # run walkthrough
```

## REPL Usage

```
╔══════════════════════════════════════════════════╗
║  Maxima Kernel (Rust)  v0.1.0                    ║
║  A Computer Algebra System                       ║
╚══════════════════════════════════════════════════╝

(%i1) factor(x^6 - 1);
(%o1) (-1+x)*(1+x)*(1+x+x^2)*(1-x+x^2)

(%i2) integrate(exp(x)*sin(x), x);
(%o2) exp(x)*(sin(x)-cos(x))/2

(%i3) solve(x^3 - 6*x^2 + 11*x - 6, x);
(%o3) [x = 1, x = 2, x = 3]

(%i4) diff(atan(x), x);
(%o4) 1/(1+x^2)

(%i5) limit((x^2-1)/(x-1), x, 1);
(%o5) 2
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate command history |
| `←` / `→` | Move cursor |
| `Home` / `End` | Jump to start/end of line |
| `Ctrl+A` / `Ctrl+E` | Jump to start/end of line |
| `Ctrl+W` | Delete word backward |
| `Ctrl+U` | Clear line |
| `Ctrl+R` | Reverse search history |
| `Ctrl+C` | Cancel current input |
| `Ctrl+D` | Exit |

### Multi-line Input

Expressions accumulate until terminated by `;` (display result) or `$` (suppress output):

```
(%i1) block([s:0],
  ...    for i:1 thru 10 do s:s+i,
  ...    s);
(%o1) 55

(%i2) f(x) := x^2 + 1$       /* suppressed */
(%i3) f(5);
(%o3) 26
```

### Syntax Highlighting

Output is color-coded when running in a terminal:
- **Cyan**: numbers
- **Yellow**: operators (`+`, `-`, `*`, `^`, `=`)
- **Blue**: functions (`sin`, `cos`, `integrate`, `solve`, ...)
- **Magenta**: booleans (`true`, `false`, `done`)
- **Bold**: constants (`%pi`, `%e`, `%i`), brackets

Set `NO_COLOR=1` to disable colors.

## Features

### Calculus
```
diff(sin(x^2), x);                       → 2*x*cos(x^2)
integrate(x*exp(x), x);                  → (x-1)*exp(x)
integrate(1/(x^2+x+1), x);              → 2*atan((1+2*x)/sqrt(3))/sqrt(3)
integrate(1/(x^4+1), x);                → log+atan with √2 coefficients
integrate(1/((x+1)*sqrt(x^2+5)), x);    → log via Euler substitution
integrate(x^n*exp(-x), x, 0, inf);      → factorial(n)
integrate(exp(-2*x^2)*cos(3*x), x, 0, inf); → Gaussian-cosine
limit(exp(-x), x, inf);                 → 0
taylor(sin(x), x, 0, 5);               → x - x^3/6 + x^5/120
```

### Differential Equations
```
ode2('diff(y,x)=x*y, y, x);                  → separable: y = %c*exp(x^2/2)
ode2('diff(y,x,2)+y=0, y, x);                → %k1*cos(x)+%k2*sin(x)
ode2('diff(y,x,2)+y=x^2, y, x);              → undetermined coeffs: +x^2-2
ode2('diff(y,x,2)+y=sin(x), y, x);           → variation of parameters (resonance)
ic2(ode2('diff(y,x,2)+y=0,y,x), x=0,y=1,'diff(y,x)=0);   → cos(x)
bc2(ode2('diff(y,x,2)+y=0,y,x), x=0,y=0, x=%pi/2,y=1);   → sin(x)
```
Every non-homogeneous particular solution is verified numerically before it
is returned; otherwise `ode2` falls back to the noun form.

### Algebra
```
expand((a+b)*(a-b));          → a^2 - b^2
factor(x^4 + x^2 + 1);       → (1+x+x^2)*(1-x+x^2)
ratsimp((x^2-1)/(x-1));      → x+1
partfrac(1/(x^2-1), x);      → 1/(2*(x-1)) - 1/(2*(x+1))
gcd(x^2-1, x^2+2*x+1);      → x+1
```

### Solving
```
solve(x^2 - 5*x + 6, x);          → [x = 2, x = 3]
solve(a*x^2 + b*x + c = 0, x);    → quadratic formula with √(b²-4ac)
solve(x^4 - 5*x^2 + 4, x);       → [x = 1, x = -1, x = 2, x = -2]
linsolve([x+y=3, 2*x-y=0], [x,y]); → [x = 1, y = 2]
```

### Summation
```
sum(k, k, 1, n);              → n*(n+1)/2  (closed form via Gosper)
sum(k^2, k, 1, n);            → closed form
sum(1/(k*(k+1)), k, 1, n);    → telescoping
sum(binomial(n,k), k, 0, n);  → 2^n
```

### Matrices
```
determinant(matrix([a,b],[c,d]));    → a*d - b*c
invert(matrix([1,2],[3,4]));         → matrix([-2,1],[3/2,-1/2])
eigenvalues(matrix([2,1],[1,2]));    → [[1,3],[1,1]]
charpoly(matrix([1,2],[3,4]), x);    → x^2 - 5*x - 2
```

### Assumptions
```
assume(x > 0);
is(x > 0);           → true
abs(x);               → x    (known positive)
forget(x > 0);
```

### Package System
```
load("mylib");                           → load and evaluate .mac file
require("mylib");                        → load only if not already loaded
setup_autoload("mylib", f1, f2);         → lazy-load on first call to f1/f2
loaded_files();                          → list loaded file paths
file_search("name");                     → find file in search paths
file_search_maxima();                    → list configured search paths
save("file.mac", var1, var2);            → write variable bindings
stringout("file.mac", expr1, expr2);     → write expressions as source
```

### LaTeX Output
```
tex(x^2/(x+1));       → "\frac{x^{2}}{1+x}"
```

## Walkthroughs

36 interactive tutorials in `walkthrough/`. Run any topic:

```sh
cargo run -- -b walkthrough/01_arithmetic.mac
cargo run -- -b walkthrough/03_calculus.mac
cargo run -- -b walkthrough/08_advanced_integration.mac
```

See [`walkthrough/README.md`](walkthrough/README.md) for the full topic list,
and [`user-manual.md`](user-manual.md) for comprehensive documentation.

## Plugin API

Extend the kernel with native Rust functions:

```rust
use maxima_eval::{Environment, NativeFn};
use maxima_core::Expr;

fn my_double(args: &[Expr], _env: &mut Environment) -> Expr {
    match &args[0] {
        Expr::Integer(n) => Expr::Integer(n * 2),
        other => Expr::call("my_double", vec![other.clone()]),
    }
}

let mut env = Environment::new();
env.register_native("my_double", my_double as NativeFn, 1, Some(1));
// Now callable from Maxima: my_double(21) → 42
```

Native functions take priority over user-defined functions and survive `kill(all)`.

### Dynamic plugins

Compiled Rust plugins (`cdylib`) can be loaded at runtime:

```maxima
load_plugin("target/debug/libmaxima_orthopoly");
legendre_p(2, x);     /* → (3/2)*x^2-1/2 */
load_plugin("target/debug/libmaxima_specfun");
erf(1.0);             /* → 0.8427... */
```

Shipped plugins: `maxima-orthopoly` (orthogonal polynomials) and
`maxima-specfun` (gamma, beta, erf, Bessel). Write your own with the
`maxima-plugin` authoring kit — copy `plugins/template` and see the
[plugin development manual](plugin-dev-manual.md).

## Build & Test

```sh
cargo build                # build all crates
cargo test                 # run 1008 unit + integration tests
cargo run                  # start REPL
```

## Project Structure

```
maxima-kernel/
├── crates/core/       Expr types, symbol interning, operators
├── crates/parser/     Tokenizer + Pratt parser (full Maxima syntax)
├── crates/eval/       Evaluator, simplifier, assumptions, limits, integration
├── crates/poly/       Sparse polynomial arithmetic, GCD, factoring, algebraic fields
├── crates/repl/       Interactive REPL with readline
├── walkthrough/       36 interactive tutorials (.mac files)
└── user-manual.md     Comprehensive user documentation
```

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

Note: This is a clean-room reimplementation. The original Maxima (Common Lisp)
is GPL-2.0, but this Rust kernel shares no code with it.
