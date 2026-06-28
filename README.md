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
║  Maxima Kernel (Rust)  v12.6.0                   ║
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
integrate(1/(x^2+1)^(3/2), x);          → x/sqrt(x^2+1)  (algebraic, Hermite reduction)
integrate(x^5/sqrt(x^3+1), x);          → elementary R·√(x³+1)  (hyperelliptic; ∫1/√(x³+1) → noun)
integrate(x^n*exp(-x), x, 0, inf);      → factorial(n)
integrate(x^(2*n)*exp(-x^2), x, 0, inf);→ (2n)!*sqrt(%pi)/(2*4^n*n!)  (parametric, Almkvist–Zeilberger)
integrate(exp(-2*x^2)*cos(3*x), x, 0, inf); → Gaussian-cosine
integrate(1/(x^2+2*x+5), x, minf, inf); → %pi/2   (residues, upper half-plane)
integrate(1/(x^2+1)^3, x, minf, inf);   → 3*%pi/8 (repeated pole, reduction)
integrate(cos(x)/(x^2+1), x, minf, inf); → %pi*exp(-1)  (Jordan's lemma)
integrate(1/(2+cos(x)), x, 0, 2*%pi);   → 2*%pi/sqrt(3) (unit-circle contour)
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
ode2('x^2*'diff(y,x,2)+x*'diff(y,x)-y=0, y, x);          → Euler: %k1*x+%k2/x
ode2('x^2*'diff(y,x,2)+x*'diff(y,x)+y=0, y, x);          → %k1*cos(log(x))+%k2*sin(log(x))
desolve('diff(y,t,2)+y=0, y(t));                         → cos(t)*y(0)+sin(t)*at('diff(y,t),t=0)
atvalue(y(t),t=0,2)$ atvalue('diff(y,t),t=0,3)$ desolve('diff(y,t,2)+y=0, y(t)); → 2*cos(t)+3*sin(t)
```
`desolve` solves linear constant-coefficient ODEs by the Laplace-transform
method (transform → solve for Y(s) → `ilt`); initial values come from `atvalue`,
otherwise stay symbolic as `y(0)`, `at('diff(y,t),t=0)`.
Every non-homogeneous particular solution is verified numerically before it
is returned; otherwise `ode2` falls back to the noun form.

### Laplace transforms
```
laplace(sin(t), t, s);           → 1/(1+s^2)
ilt(1/(s^2+1), s, t);            → sin(t)
ilt(1/(s^2-1), s, t);            → (1/2)*exp(t)-(1/2)*exp(-t)   (sinh)
ilt(s/(s^2+2*s+5), s, t);        → exp(-t)*(cos(2*t)-sin(2*t)/2) (damped)
ilt(6/((s+1)*(s+2)*(s+3)), s, t); → 3*exp(-t)-6*exp(-2*t)+3*exp(-3*t)
```
Inverse Laplace handles a general rational `F(s)=N/D`: `D` is factored over ℚ,
the partial-fraction numerators are solved exactly, and each term is inverted by
its transform pair (real poles → `t^j·e^(at)`, irreducible quadratics → damped
`sin`/`cos`). Verified by the `laplace(ilt(F))=F` round-trip.

### Algebra
```
expand((a+b)*(a-b));          → a^2 - b^2
factor(x^4 + x^2 + 1);       → (1+x+x^2)*(1-x+x^2)
factor(a^2 - b^2);            → (a-b)*(a+b)      (multivariate)
factor(x^3 - y^3);            → (x-y)*(x^2+x*y+y^2)
ratsimp((x^2-1)/(x-1));      → x+1
partfrac(1/(x^2-1), x);      → 1/(2*(x-1)) - 1/(2*(x+1))
gcd(x^2-1, x^2+2*x+1);      → x+1
gcd(x^2-y^2, x-y);            → x-y            (multivariate)
```

The multivariate `gcd`/`factor` use Kronecker substitution to the univariate
engine, with every factor exact-division-verified (correct, never wrong).

### Solving
```
solve(x^2 - 5*x + 6, x);          → [x = 2, x = 3]
solve(x^2 + 1, x);               → [x = %i, x = -%i]            (complex)
solve(x^2 - 2, x);               → [x = sqrt(2), x = -sqrt(2)]  (radical)
solve(x^4 - 4*x^2 + 1, x);       → [±sqrt(2±sqrt(3))]           (biquadratic)
solve(x^3 - 2, x);               → [2^(1/3), 2^(1/3)·ω, 2^(1/3)·ω²]  (Cardano)
solve(x^3 + x + 1, x);           → real radical root + 2 complex (general Cardano)
solve(x^3 - 3*x + 1, x);         → 3 real roots in complex radicals (casus irreducibilis)
solve(x^4 + x + 1, x);           → 4 roots (Ferrari resolvent cubic)
solve(x^5 - x - 1, x);           → [x = rootof(x^5-x-1, x, 1), …, x, 5)]  (no radicals)
float(rootof(x^5-x-1, x, 1));    → 1.167303978261419   (real root first)
bfloat(rootof(x^5-x-1, x, 1));   → 1.16730397826141868425604589985b0  (Newton-refined)
linsolve([x+y=a, x-y=b], [x,y]);  → [x = (a+b)/2, y = (a-b)/2]   (symbolic)
```
`solve` factors over ℚ then solves each factor by radicals: quadratic, general
cubic (Cardano, incl. casus irreducibilis via complex radicals), and general
quartic (Ferrari). Every radical root is verified numerically (|p(r)| < 1e-6
over ℂ). Factors with no radical solution (e.g. a general quintic) return
`rootof(p, x, k)` nouns — all roots via Durand–Kerner, real roots first — which
`float`/`bfloat` evaluate (real roots refined to full precision by Newton).

### Root analysis
```
sturm(x^3-2*x-5, x);     → [x^3-2*x-5, 3*x^2-2, (4/3)*x+5, -643/16]  (Sturm chain)
nroots(x^5-x-1);         → 1     (distinct real roots over the Cauchy bound)
nroots(x^4+1);           → 0
realroots(x^2-2);        → [x = -97184015997/68719476736, x = 97184015997/68719476736]  (exact rationals within eps)
realroots(x^3-x);        → [x = -1, x = 0, x = 1]   (exact rational roots)
```

### Summation & Creative Telescoping
```
nusum(k*k!, k, 1, n);            → (n+1)!-1     (Gosper indefinite)
nusum(2^k, k, 1, n);             → 2^(n+1)-2
sum(k^3, k, 1, n);               → (n*(n+1)/2)^2
sum(1/(k*(k+1)), k, 1, n);       → 1-1/(n+1)    (telescoping)
sum(binomial(n,k), k, 0, n);     → 2^n
sum(k*binomial(n,k), k, 0, n);   → n*2^(n-1)
sum(binomial(n,k)^2, k, 0, n);   → (2n)!/(n!)^2  (= binomial(2n,n))
sum(1/k, k, 1, n);               → harmonic(n)        (harmonic number)
sum(1/k^2, k, 1, n);             → harmonic(n,2)      (generalized)
sum(1/k^2, k, 1, inf);           → %pi^2/6            (ζ(2); ζ(p) for p≥2)
sum(k*x^k, k, 1, inf);           → x/(1-x)^2          (generating function)
sum(k*(1/2)^k, k, 1, inf);       → 2                  (numeric base, verified)
```
Definite hypergeometric sums are resolved by order-1 recurrence detection
(integer & half-integer shifts), every closed form numerically verified.
Harmonic sums `Σ 1/k^p` give `harmonic(n)` / `harmonic(n,p)` (→ ζ(p) when
infinite); generating-function sums `Σ p(k)·xᵏ` give a rational in x, numerically
verified before return (divergent series stay nouns).

For D-finite sequences with **no** elementary closed form, `find_recurrence`
returns the linear P-recurrence `[c_0(n), …, c_J(n)]` (meaning Σ c_j(n)·S(n+j)=0):
```
find_recurrence(sum(binomial(n,k)^3,k,0,n), n);  → Franel: [-8-16n-8n², -16-21n-7n², 4+4n+n²]
find_recurrence(sum(binomial(n,k)*binomial(n+k,k),k,0,n), n);  → central Delannoy
```
`solve_rec` closes a C-finite (constant-coefficient) recurrence to a closed form
via its characteristic roots; `gosper_certificate` returns the Gosper/WZ
certificate R(k) proving an indefinite sum telescopes (verified symbolically):
```
solve_rec(3*2^n - 5, n);         → -5+3*2^n         (roots 2, 1)
gosper_certificate(k*k!, k);     → 1/k              (T(k)=k!, so Σ k·k! = (n+1)!-1)
```

### Matrices
```
determinant(matrix([a,b],[c,d]));    → a*d - b*c
invert(matrix([1,2],[3,4]));         → matrix([-2,1],[3/2,-1/2])
charpoly(matrix([1,2],[3,4]), x);    → x^2 - 5*x - 2
rank(matrix([a,b],[2*a,2*b]));       → 1                  (exact, incl. symbolic)
rref(matrix([1,2,3],[4,5,6]));       → matrix([1,0,-1],[0,1,2])
triangularize(matrix([1,2],[3,4]));  → matrix([1,2],[0,-2])
nullspace(matrix([1,2],[2,4]));      → [matrix([-2],[1])]
eigenvalues(matrix([2,1],[1,2]));    → [[1,3],[1,1]]
eigenvalues(matrix([0,1],[1,1]));    → golden ratio (1±sqrt(5))/2
eigenvalues(matrix([0,-1],[1,0]));   → [[%i,-%i],[1,1]]
```
`rank`/`rref`/`triangularize`/`nullspace` use exact Gaussian elimination;
`eigenvalues` solves the characteristic polynomial by radicals (irrational/complex).

### Numerics
```
find_root(x^2-2, x, 0, 2);           → 1.414213562373095   (bisection)
find_root(cos(x)-x, x, 0, 1);        → 0.73908513321516
romberg(sin(x), x, 0, %pi);          → 2.0                 (quadrature)
quad_qags(exp(-x^2), x, 0, 1);       → 0.746824132812427
rk(-y, y, 1, [t,0,1,0.5]);           → RK4 trajectory [[t,y],...]
zeta(2);                             → %pi^2/6
zeta(3.0);                           → 1.202056903159729   (Apéry)
lambert_w(1.0);                      → 0.567143290409784   (Omega)
polylog(2, 1);                       → %pi^2/6
```

### Arbitrary-precision bigfloats
```
fpprec: 40;  bfloat(%pi);    → 3.141592653589793238462643383279502884197b0
bfloat(sqrt(2));             → 1.414213562373095048801688724209698078569b0
bfloat(%pi + %e);            → 5.859874482048838473822930854632165381954b0
bfloat(%pi)*2;               → contagion: arithmetic with a bigfloat stays bigfloat
```
`bfloat(expr)` evaluates the whole expression to `fpprec` digits via an
arbitrary-precision backend (astro-float): constants (`%pi`, `%e`, `%phi`,
`%gamma`), arithmetic, powers, and elementary functions. A bigfloat mixed with
other numbers in `+`/`*`/`^` folds at the widest operand precision.

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

### Help System
```
help();                         → list documented functions
help("factor");                 → full help page for factor
help("factor", "usage");        → just the usage section
```

Help pages are stored in `crates/eval/src/help.toml` and embedded into the
binary. Every built-in function has an entry (295+ functions), with full rich
pages for the core calculus, algebra, linear-algebra, list/set, and special-
function commands. Each entry supports title, description, usage, arguments,
details, return value, references, authors, and aliases.

## Walkthroughs

41 interactive tutorials in `walkthrough/`. Run any topic:

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
legendre_q(2, x);     /* → ((3*x^2-1)/4)*log((1+x)/(1-x)) - 3*x/2 */
load_plugin("target/debug/libmaxima_specfun");
erf(1.0);             /* → 0.8427... */
bessel_y(1, 1.0);     /* → -0.7812... */
bessel_k(0, 1.0);     /* → 0.4210... */
```

Shipped plugins:
- `maxima-orthopoly` — orthogonal polynomials: `legendre_p`, `legendre_q`,
  `chebyshev_t`, `chebyshev_u`, `hermite`, `laguerre`, `gen_laguerre`,
  `ultraspherical`, `jacobi_p`.
- `maxima-specfun` — special functions: `gamma`, `log_gamma`, `beta`, `erf`,
  `erfc`, `bessel_j`, `bessel_i`, `bessel_y`, `bessel_k`.

Write your own with the `maxima-plugin` authoring kit — copy
`plugins/template` and see the [plugin development manual](plugin-dev-manual.md).

## Build & Test

```sh
cargo build                # build all crates
cargo test                 # run 1116 unit + integration tests
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
