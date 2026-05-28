# MaximaST: A Rust Maxima Kernel

A computer algebra system kernel rewritten in Rust, with syntax compatible with [Maxima](https://maxima.sourceforge.io/).

This is an AI-driven, human-in-the-loop auto-research and vibe-coding project. It is highly unstable and intended for experimentation only.

Please note that many edge cases remain untested. Use at your own risk.

## Quick Start

```sh
cargo run                                        # start the REPL
cargo run -- -e "integrate(1/(x^4+1), x);"      # evaluate an expression
cargo run -- -b walkthrough/03_calculus.mac     # run a walkthrough
```

## Useful Resources

- `CLAUDE.md` describes the development harness. In spirit, it is a rewrite of Karpathy-style rules of thumb.
- `rules.md` documents the development rules used across roughly 90 pull requests.
- `skills.md` summarizes the capabilities and intended skill set of MaximaST.
- `./walkthrough/` contains walkthroughs for quick start and general orientation.

Most source files are reasonably self-explanatory and include adequate—though still incomplete—tests. A larger test suite of 10,000+ cases, based on known integral tables and textbooks/manuscripts, is planned. **Help is welcome.**

You can also explore the auto-research materials in `./research/`:

- `manuscript`: technical report
- `survey`: algorithm survey
- `integralformulalist.toml`: a small but useful test suite

I also attached all sprints designs for harness developers/researchers in `./spint`:

- `spint-v1.0`: Initial, go/no-go, detailed
...
- `spint-v5.0`: New Functions for sets, strings, numtheory, expr manip, poly analysis, logcontract/expand, Laplace transforms, ODE solver (update), complex %i, trig table, matrix +/-, display, partfrac quadratics, non-homog ODE, Sturm, pattern matching, bfloat, plot2d (Naive RUST + gnuplot script)

It is noticable, all sprint files are provided ASIS. There is no modification manually.

## REPL Usage

```text
╔══════════════════════════════════════════════════╗
║  MaximaST   v0.1.0                               ║
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
| `Home` / `End` | Jump to the start/end of the line |
| `Ctrl+A` / `Ctrl+E` | Jump to the start/end of the line |
| `Ctrl+W` | Delete the previous word |
| `Ctrl+U` | Clear the current line |
| `Ctrl+R` | Reverse-search command history |
| `Ctrl+C` | Cancel the current input |
| `Ctrl+D` | Exit |

### Multi-line Input

Expressions accumulate until terminated by `;` (show result) or `$` (suppress output):

```text
(%i1) block([s:0],
  ...    for i:1 thru 10 do s:s+i,
  ...    s);
(%o1) 55

(%i2) f(x) := x^2 + 1$       /* suppressed */
(%i3) f(5);
(%o3) 26
```

### Syntax Highlighting

When running in a terminal, output is color-coded:

- **Cyan**: numbers
- **Yellow**: operators (`+`, `-`, `*`, `^`, `=`)
- **Blue**: functions (`sin`, `cos`, `integrate`, `solve`, ...)
- **Magenta**: booleans (`true`, `false`, `done`)
- **Bold**: constants (`%pi`, `%e`, `%i`) and brackets

Set `NO_COLOR=1` to disable colors.

## Features

### Calculus

```text
diff(sin(x^2), x);                           → 2*x*cos(x^2)
integrate(x*exp(x), x);                      → (x-1)*exp(x)
integrate(1/(x^2+x+1), x);                   → 2*atan((1+2*x)/sqrt(3))/sqrt(3)
integrate(1/(x^4+1), x);                     → log + atan with √2 coefficients
integrate(1/((x+1)*sqrt(x^2+5)), x);         → log via Euler substitution
integrate(x^n*exp(-x), x, 0, inf);           → factorial(n)
integrate(exp(-2*x^2)*cos(3*x), x, 0, inf);  → Gaussian-cosine integral
limit(exp(-x), x, inf);                      → 0
taylor(sin(x), x, 0, 5);                     → x - x^3/6 + x^5/120
```

### Algebra

```text
expand((a+b)*(a-b));               → a^2 - b^2
factor(x^4 + x^2 + 1);             → (1+x+x^2)*(1-x+x^2)
ratsimp((x^2-1)/(x-1));            → x+1
partfrac(1/(x^2-1), x);            → 1/(2*(x-1)) - 1/(2*(x+1))
gcd(x^2-1, x^2+2*x+1);             → x+1
```

### Solving

```text
solve(x^2 - 5*x + 6, x);               → [x = 2, x = 3]
solve(a*x^2 + b*x + c = 0, x);         → quadratic formula with √(b²-4ac)
solve(x^4 - 5*x^2 + 4, x);             → [x = 1, x = -1, x = 2, x = -2]
linsolve([x+y=3, 2*x-y=0], [x,y]);     → [x = 1, y = 2]
```

### Summation

```text
sum(k, k, 1, n);                    → n*(n+1)/2  (closed form via Gosper)
sum(k^2, k, 1, n);                  → closed form
sum(1/(k*(k+1)), k, 1, n);          → telescoping
sum(binomial(n,k), k, 0, n);        → 2^n
```

### Matrices

```text
determinant(matrix([a,b],[c,d]));   → a*d - b*c
invert(matrix([1,2],[3,4]));        → matrix([-2,1],[3/2,-1/2])
eigenvalues(matrix([2,1],[1,2]));   → [[1,3],[1,1]]
charpoly(matrix([1,2],[3,4]), x);   → x^2 - 5*x - 2
```

### Assumptions

```text
assume(x > 0);
is(x > 0);      → true
abs(x);         → x    (known positive)
forget(x > 0);
```

### Package System

```text
load("mylib");                       → load and evaluate a .mac file
require("mylib");                    → load only if not already loaded
setup_autoload("mylib", f1, f2);     → lazy-load on first use of f1/f2
loaded_files();                      → list loaded file paths
file_search("name");                 → search for a file in configured paths
file_search_maxima();                → list configured search paths
save("file.mac", var1, var2);        → write variable bindings
stringout("file.mac", expr1, expr2); → write expressions as source
```

### LaTeX Output

```text
tex(x^2/(x+1));       → "\frac{x^{2}}{1+x}"
```

## Walkthroughs

Interactive tutorials are available in `walkthrough/`. Run any topic with:

```sh
cargo run -- -b walkthrough/01_arithmetic.mac
cargo run -- -b walkthrough/03_calculus.mac
cargo run -- -b walkthrough/08_advanced_integration.mac
```

See [`walkthrough/README.md`](walkthrough/README.md) for the full topic list, and [`user-manual.md`](user-manual.md) for more complete documentation.

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

Native functions take precedence over user-defined functions and persist across `kill(all)`.

## Build & Test

```sh
cargo build                # build all crates
cargo test                 # run 920 unit and integration tests
cargo run                  # start the REPL
```

## Project Structure

```text
maxima-kernel/
├── crates/core/       Expr types, symbol interning, operators
├── crates/parser/     Tokenizer + Pratt parser (full Maxima syntax)
├── crates/eval/       Evaluator, simplifier, assumptions, limits, integration
├── crates/poly/       Sparse polynomial arithmetic, GCD, factoring, algebraic fields
├── crates/repl/       Interactive REPL with readline
├── walkthrough/       Interactive tutorials (.mac files)
└── user-manual.md     Comprehensive user documentation
```

## License

1. The Maxima-compatibility test cases in `./tests/` are GPL-2.0, following Maxima. If you wan to integrate the test in the project, CHANGE TO GPL license!!! Removing it only affects the compatibility test (somewhat).

2. The main codebase (kernel and REPL) is licensed under either of the following, at your option:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

Note: This is a clean-room reimplementation. The original Maxima implementation is written in Common Lisp and distributed under GPL-2.0, but this Rust kernel shares no code with it.

## Contributors

- Yifan Yang <yfyang.86 hotmail>
- Claude Code

## Thanks

Thanks to all contributors to Maxima, and to the researchers and engineers who have advanced computer algebra systems.
