# Maxima Kernel (Rust) — User Manual

A computer algebra system kernel rewritten in Rust, compatible with
[Maxima](https://maxima.sourceforge.io/) syntax.

## Getting Started

### Installation

```sh
cd maxima-kernel
cargo build --release
```

The binary is at `target/release/maxima-repl`.

### Running

```sh
# Interactive REPL
maxima-repl

# Evaluate a single expression
maxima-repl -e "factor(x^6-1);"

# Run a script file
maxima-repl -b script.mac
maxima-repl script.mac

# Pipe from stdin
echo "diff(sin(x), x);" | maxima-repl
```

### Command-line Options

| Option | Description |
|--------|-------------|
| `-e <expr>` | Evaluate expression and exit |
| `-b <file>` | Run file in batch mode |
| `-q` | Suppress startup banner |
| `--very-quiet` | Suppress banner and prompts |
| `-v` | Print version |
| `-h` | Print help |

### REPL Features

- **Tab completion**: type a prefix and press Tab to complete function
  names, keywords, and constants
- **History**: use Up/Down arrows to navigate previous inputs
- **Multi-line input**: expressions accumulate until terminated by `;`
  (display) or `$` (suppress)
- **Syntax highlighting**: color-coded output (disable with `NO_COLOR=1`)

---

## Help System

The built-in `help(...)` command displays documentation for supported
functions. Help pages are stored in `crates/eval/src/help.toml` and embedded
into the binary.

```
help();                         → list documented functions
help("factor");                 → full help page for factor
help("factor", "usage");        → just the usage section
help("factor", "description");  → specific section
```

Supported sections for the second argument: `title`, `description`, `usage`,
`arguments`, `details`, `value`, `references`, `authors`.

Each help entry in `help.toml` has the following fields:

- `name` — function name
- `alias` — array of alternative names
- `title` — short headline
- `description` — overview (markdown accepted)
- `usage` — syntax examples (markdown accepted)
- `arguments` — argument descriptions (markdown accepted)
- `details` — extended explanation (markdown accepted)
- `value` — return value description (markdown accepted)
- `references` — array of URLs
- `authors` — array of authors

---

## Arithmetic

Exact rational arithmetic — no floating-point rounding unless requested.

```
2 + 3;              → 5
1/3 + 1/6;          → 1/2
2^(-3);             → 1/8
100!;               → 933262154439...  (big integer)
```

### Float Conversion

```
float(%pi);          → 3.141592653589793
float(sqrt(2));      → 1.414213562373095
float(1/7);          → 0.142857142857143
```

### Number Theory

```
gcd(12345678, 87654321);   → 9
primep(127);               → true
binomial(20, 10);          → 184756
mod(17, 5);                → 2
```

### Rounding

```
floor(7/2);        → 3
ceiling(7/2);      → 4
truncate(-7/2);    → -3
round(1/2);        → 0     (banker's rounding: half to even)
round(3/2);        → 2
```

---

## Algebra

### Expansion and Factoring

```
expand((x+1)^6);                → 1+6*x+15*x^2+20*x^3+15*x^4+6*x^5+x^6
factor(x^6-1);                  → (-1+x)*(1+x)*(1+x+x^2)*(1-x+x^2)
factor(x^4+x^2+1);             → (1+x+x^2)*(1-x+x^2)
```

### Simplification

```
ratsimp((x^2-1)/(x-1));        → x+1
partfrac(1/(x*(x+1)^2), x);    → partial fraction decomposition
trigsimp(sin(x)^2+cos(x)^2);   → 1
trigexpand(sin(a+b));           → sin(a)*cos(b)+cos(a)*sin(b)
```

### Polynomial GCD

```
gcd(x^4-1, x^6-1);             → polynomial GCD
```

### Substitution

```
subst(3, x, x^3+2*x+1);       → 34
```

---

## Calculus

### Differentiation

```
diff(sin(x^2), x);              → 2*x*cos(x^2)
diff(exp(x)*log(x), x);         → exp(x)*x^(-1)+exp(x)*log(x)
diff(x^5, x, 3);                → 60*x^2   (third derivative)
```

### Indefinite Integration

```
integrate(x^3, x);              → (1/4)*x^4
integrate(sin(x)^2, x);         → -(1/2)*cos(x)*sin(x)+(1/2)*x
integrate(exp(x)*sin(x), x);    → exp(x)*(sin(x)-cos(x))/2
integrate(1/(x^4+1), x);        → log+atan with √2 coefficients
integrate(log(x)^3/x, x);       → (1/4)*log(x)^4
```

### Definite Integration

```
integrate(x^2, x, 0, 1);           → 1/3
integrate(sin(x), x, 0, %pi);      → 2
integrate(exp(-x), x, 0, inf);     → 1
integrate(1/(x^2+1), x, minf, inf);→ %pi
integrate(x^n*exp(-x), x, 0, inf); → factorial(n)  (Gamma function)
```

### Limits

```
limit(sin(x)/x, x, 0);             → 1
limit((1+1/x)^x, x, inf);         → exp(1)
limit(exp(x)/x^100, x, inf);      → inf
limit(exp(x), x, minf);           → 0
```

Directional limits:

```
limit(sin(x)/abs(x), x, 0, plus);  → 1
limit(sin(x)/abs(x), x, 0, minus); → -1
limit(sin(x)/abs(x), x, 0);        → und  (left ≠ right)
```

### Taylor Series

```
taylor(sin(x), x, 0, 7);       → x - x^3/6 + x^5/120 - x^7/5040
taylor(exp(x), x, 0, 5);       → 1 + x + x^2/2 + x^3/6 + ...
```

### Ordinary Differential Equations

`ode2(eqn, y, x)` solves first- and second-order ODEs. Use `'diff` (quoted)
so the derivative is not evaluated before the solver sees it.

```
ode2('diff(y,x) = x*y, y, x);            → y = %c*exp(x^2/2)   (separable)
ode2('diff(y,x) + y = 0, y, x);          → y = %c*exp(-x)      (linear)
ode2('diff(y,x,2) - y = 0, y, x);        → %k1*exp(x)+%k2*exp(-x)
ode2('diff(y,x,2) + y = 0, y, x);        → %k1*cos(x)+%k2*sin(x)
ode2('diff(y,x,2) + 4*'diff(y,x) + 4*y = 0, y, x); → (%k1+%k2*x)*exp(-2*x)
```

Non-homogeneous equations are solved by undetermined coefficients (polynomial,
exponential, sine/cosine forcing) and, as a general fallback, by variation of
parameters:

```
ode2('diff(y,x,2) + y = x^2, y, x);      → %k1*cos(x)+%k2*sin(x)+x^2-2
ode2('diff(y,x,2) - 3*'diff(y,x) + 2*y = x, y, x); → ...+x/2+3/4
ode2('diff(y,x,2) - y = exp(x), y, x);   → ...+x*exp(x)/2  (resonance)
ode2('diff(y,x,2) + y = sin(x), y, x);   → solved by variation of parameters
```

Each particular solution is verified numerically before being returned; if it
cannot be confirmed, `ode2` returns the unevaluated noun form.

Initial and boundary conditions specialise the constants:

```
ic1(ode2('diff(y,x)=x, y, x), x=0, y=1);                 → y = 1 + x^2/2
ic2(ode2('diff(y,x,2)+y=0,y,x), x=0, y=1, 'diff(y,x)=0); → y = cos(x)
ic2(ode2('diff(y,x,2)+y=0,y,x), x=0, y=0, 'diff(y,x)=1); → y = sin(x)
bc2(ode2('diff(y,x,2)+y=0,y,x), x=0, y=0, x=%pi/2, y=1); → y = sin(x)
```

---

## Solving Equations

```
solve(x^2-5*x+6, x);              → [x = 2, x = 3]
solve(a*x^2+b*x+c=0, x);          → quadratic formula with √(b²-4ac)
solve(x^4-5*x^2+4, x);            → [x = 1, x = -1, x = 2, x = -2]
```

### Linear Systems

```
linsolve([x+y=3, 2*x-y=0], [x, y]);         → [x = 1, y = 2]
linsolve([x+y+z=6, x-y=2, 2*y+z=5], [x,y,z]); → [x = 3, y = 1, z = 3]
```

---

## Summation

Closed-form evaluation via the Gosper algorithm:

```
sum(k, k, 1, n);                 → n*(n+1)/2
sum(k^2, k, 1, n);              → closed form
sum(1/(k*(k+1)), k, 1, n);      → telescoping
sum(binomial(n,k), k, 0, n);    → 2^n
```

Numeric sums:

```
sum(k, k, 1, 100);              → 5050
sum(k^2, k, 1, 10);             → 385
```

---

## Matrices

### Construction and Access

```
A : matrix([1,2], [3,4]);
A[1,2];                          → 2
A[2,1];                          → 3
A[1];                            → [1,2]  (first row)
```

### Operations

```
determinant(matrix([a,b],[c,d]));    → a*d-b*c
invert(matrix([1,2],[3,4]));         → inverse matrix
transpose(matrix([1,2,3],[4,5,6]));  → transposed matrix
charpoly(matrix([1,2],[3,4]), x);    → characteristic polynomial
eigenvalues(matrix([2,1],[1,2]));    → [[1,3],[1,1]]
```

### Matrix Power

```
M : matrix([1,1],[1,0]);
M^^2;            → matrix([2,1],[1,1])
M^^10;           → matrix([89,55],[55,34])   (Fibonacci!)
M^^0;            → identity matrix
```

---

## Assumptions

```
assume(x > 0);
is(x > 0);          → true
abs(x);              → x      (known positive)
forget(x > 0);
facts();             → list current assumptions
```

---

## Programming

### Function Definition

```
f(x) := x^2 + 1;
f(3);                            → 10
```

### Block and Local Variables

```
block([s:0], for i:1 thru 10 do s:s+i, s);   → 55
```

### Loops

```
for i : 1 thru 5 do print(i);
for i from 0 thru 10 do print(i^2);     /* "from" syntax also supported */
while n > 0 do n : n - 1;
for x in [1,2,3] do print(x^2);
```

### Lists

```
L : [1, 2, 3, 4, 5];
first(L);             → 1
second(L);            → 2
last(L);              → 5
rest(L);              → [2, 3, 4, 5]
endcons(6, L);        → [1, 2, 3, 4, 5, 6]
length(L);            → 5
reverse(L);           → [5, 4, 3, 2, 1]
sort([5,3,1,4,2]);    → [1, 2, 3, 4, 5]
append([1,2],[3,4]);  → [1, 2, 3, 4]
makelist(k^2, k, 1, 5);  → [1, 4, 9, 16, 25]
map(lambda([x], x^2), [1,2,3]);  → [1, 4, 9]
```

### Lambda Functions

```
apply(lambda([x], x^2+1), [5]);   → 26
```

---

## File I/O and Packages

### Loading Files

```
load("mylib.mac");                  → load and evaluate
require("mylib.mac");               → load only if not already loaded
```

### Autoload

```
setup_autoload("mylib", f1, f2);    → register lazy loading
f1(x);                              → triggers load of mylib
```

### Search Paths

```
file_search("name");                → find file in search paths
file_search_maxima();               → list configured search paths
loaded_files();                     → list loaded files
```

### Saving

```
save("file.mac", var1, var2);       → write variable bindings
stringout("file.mac", expr1);       → write expressions as source
printfile("file.txt");              → display file contents
```

---

## LaTeX Output

```
tex(x^2/(x+1));                    → LaTeX string
tex(integrate(sin(x), x));         → LaTeX string
tex(matrix([a,b],[c,d]));          → LaTeX matrix
```

---

## Output Format

Expressions are printed with explicit parentheses to avoid ambiguity:

```
x^(-1)       not  x^-1       (negative exponents)
x*(1/2)      not  x*1/2      (fractional factors)
-b           not  -1*b       (negation)
x^(1/2)      not  x^1/2      (fractional exponents)
```

---

## Plugin API (Rust developers)

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
```

**Dispatch order**: native → user-defined → lambda → autoload → noun form.
Native functions survive `kill(all)`.

---

## Walkthroughs

41 interactive tutorials in `walkthrough/`:

| # | File | Topic |
|---|------|-------|
| 01 | `01_arithmetic.mac` | Integers, rationals, floats, big numbers |
| 02 | `02_algebra.mac` | Expand, factor, ratsimp, partfrac, trig |
| 03 | `03_calculus.mac` | Differentiation, integration, Taylor series |
| 04 | `04_solving.mac` | Polynomial roots, quadratic formula, linear systems |
| 05 | `05_limits.mac` | Limits at points, infinity, indeterminate forms |
| 06 | `06_matrices.mac` | Determinant, inverse, eigenvalues, charpoly |
| 07 | `07_summation.mac` | Closed-form sums, binomials, products |
| 08 | `08_advanced_integration.mac` | Algebraic integrands, Gaussian, Laplace |
| 09 | `09_assumptions.mac` | assume/forget, abs simplification, boolean logic |
| 10 | `10_programming.mac` | Functions, lambda, lists, makelist |
| 11 | `11_file_io.mac` | load, require, save, autoload |
| 12 | `12_plugin_api.mac` | NativeFn Rust plugin interface |
| 13 | `13_latex_output.mac` | tex() for LaTeX rendering |
| 14 | `14_matrix_applications.mac` | Matrix power, Fibonacci, indexing |
| 15 | `15_game_solver.mac` | 24-game solver (recursive programming) |
| 16 | `16_number_theory.mac` | floor, ceiling, round, mod, gcd, primep |
| 17 | `17_sets.mac` | {}-syntax, union, intersection, powerset |
| 18 | `18_strings.mac` | slength, split, substring, ssearch, parse_string |
| 19 | `19_number_theory.mac` | ifactors, totient, fibonacci, CRT, Jacobi |
| 20 | `20_laplace.mac` | Laplace transforms and inverse (ilt) |
| 21 | `21_ode.mac` | ODE solver (ode2), undetermined coeffs, variation of params, ic2/bc2 |
| 22 | `22_complex.mac` | Complex numbers: %i, realpart, conjugate, rectform |
| 23 | `23_trig_special.mac` | Exact trig values: sin(%pi/6), cos(%pi/4), ... |
| 24 | `24_matrix_arithmetic.mac` | Matrix +/-, scalar*, dot product |
| 25 | `25_partfrac_advanced.mac` | Partial fractions with irreducible quadratics |
| 26 | `26_realroots.mac` | Sturm chains: nroots, realroots |
| 27 | `27_pattern_matching.mac` | matchdeclare, defrule, apply1 |
| 28 | `28_bfloat.mac` | Floating-point evaluation (bfloat) |
| 29 | `29_plotting.mac` | plot2d (SVG), gnuplot_script |
| 30 | `30_ac_matching.mac` | AC pattern matching: commutative sums/products |
| 31 | `31_symbolic_poly.mac` | Symbolic-coefficient resultant & discriminant |
| 32 | `32_residues.mac` | Residues at simple, complex, and higher-order poles |
| 33 | `33_trig_advanced.mac` | trigrat, extended trigreduce, halfangles |
| 34 | `34_rust_plugins.mac` | Dynamic Rust plugins: load_plugin, authoring kit |
| 35 | `35_orthopoly.mac` | Orthogonal polynomials plugin: Legendre P/Q, Chebyshev, Hermite, Laguerre, Jacobi, Gegenbauer |
| 36 | `36_specfun.mac` | Special functions plugin: gamma, beta, erf, Bessel J/I/Y/K |
| 37 | `37_eight_queens.mac` | Eight queens: permutations + diagonal filter, recursive backtracking |
| 38 | `38_queens_visualization.mac` | Eight-queens visualization: ASCII boards, scoreboard, reflection symmetry |
| 39 | `39_polynomial_systems.mac` | Polynomial systems: Gröbner basis, polysys_solve, eliminate, ideal arithmetic |
| 40 | `40_sudoku_visualize.mac` | Sudoku visualization: 9x9 board rendering, validation, and candidate digits |
| 41 | `41_sudoku_solver.mac` | Sudoku solver: recursive backtracking, solution counting, and a 4x4 demo |

Run any tutorial:

```sh
maxima-repl -b walkthrough/03_calculus.mac
```

---

## Project Structure

```
maxima-kernel/
├── crates/core/       Expr types, symbol interning, operators
├── crates/parser/     Tokenizer + Pratt parser (full Maxima syntax)
├── crates/eval/       Evaluator, simplifier, assumptions, limits, integration
├── crates/poly/       Sparse polynomial arithmetic, GCD, factoring, algebraic fields
├── crates/repl/       Interactive REPL with readline and tab completion
└── walkthrough/       41 interactive tutorials (.mac files)
```

---

## Version History

| Version | Tests | Key milestone |
|---------|-------|---------------|
| v1.0 | 326 | Parser, evaluator, simplifier |
| v2.0 | 777 | Hermite reduction, Risch tower, Gruntz limits |
| v3.0 | 777 | Algebraic fields, classic Gruntz, Zeilberger |
| v4.0 | 794 | CLI, PolyAlg, algebraic integration, benchmarks |
| v5.0 | 822 | Plugin API, package system, walkthroughs, bug fixes |
| v6.0 | 1008 | AC pattern matching, residues, advanced trig, ODE (variation of parameters, ic2/bc2) |
| v7.0 | 1021 | Dynamic Rust plugin toolchain (load_plugin); orthopoly + specfun plugins |
| v7.1 | 1116 | `legendre_q` in orthopoly; `bessel_y`/`bessel_k` in specfun; `cargo test` warning-free |
| v7.2 | 1116 | Built-in `help(...)` documentation system; 41 walkthroughs with Sudoku demos |

## License

Dual-licensed under MIT and Apache-2.0.
