# Maxima Kernel — Rust Plugin Development Manual

How to write, build, load, and test a Rust plugin for the Maxima kernel.

Plugins are compiled `cdylib` libraries (`.so` / `.dylib` / `.dll`) that
register native functions callable from Maxima. They are loaded at runtime
with `load_plugin("path")`. The shipped plugins — `maxima-orthopoly` and
`maxima-specfun` — are worked examples; copy `plugins/template` to start a
new one.

---

## 1. Quick start

```sh
# from maxima-kernel/
cp -r plugins/template plugins/myplugin
$EDITOR plugins/myplugin/Cargo.toml   # rename the package
$EDITOR plugins/myplugin/src/lib.rs   # write your functions
# add "plugins/myplugin" to the workspace members in Cargo.toml
cargo build -p maxima-myplugin
```

```maxima
(%i1) load_plugin("target/debug/libmaxima_myplugin");
(%o1) true
(%i2) plugin_double(21);
(%o2) 42
```

A minimal complete plugin:

```rust
use maxima_plugin::{maxima_plugin, Expr, Environment, guard};

fn plugin_double(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("plugin_double", args, || match args.first() {
        Some(Expr::Integer(n)) => Expr::int(n * 2),
        _ => Expr::call("plugin_double", args.to_vec()),
    })
}

maxima_plugin!(register = |env| {
    env.register_native("plugin_double", plugin_double, 1, Some(1));
});
```

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]
[dependencies]
maxima-plugin = { path = "../../crates/plugin-sdk" }
```

---

## 2. Anatomy of a plugin

### 2.1 The three exported symbols

Every plugin must export three `#[no_mangle] extern "C"` functions. The
`maxima_plugin!` macro generates all of them — you should not write them by
hand:

| Symbol | Purpose |
|--------|---------|
| `maxima_plugin_abi` | Returns the ABI/version string; the host rejects a mismatch. |
| `maxima_plugin_set_interner` | Adopts the host's symbol table (see §5). |
| `maxima_plugin_register` | Runs your registration closure once at load. |

### 2.2 The registration closure

```rust
maxima_plugin!(register = |env| {
    env.register_native("name", fn_ptr, min_args, max_args);
    // ... register as many as you like
});
```

`register_native(name, func, min_args, max_args)`:
- `name: &str` — the Maxima-visible function name.
- `func` — a `fn(&[Expr], &mut Environment) -> Expr` (a plain function
  pointer, not a closure).
- `min_args: usize`, `max_args: Option<usize>` — arity bounds; `None` means
  unbounded. The host returns the noun form if the call is out of range, so
  your function only runs with an acceptable argument count.

The closure must not capture variables (it is coerced to a `fn` pointer).

### 2.3 A native function

```rust
fn my_fn(args: &[Expr], env: &mut Environment) -> Expr {
    guard("my_fn", args, || {
        // ... compute and return an Expr
    })
}
```

- Always wrap the body in `guard` (§4).
- Return a fully-formed `Expr`. The host does **not** auto-simplify a native
  result, so build it cleanly (use `simplify` or `meval`, §3.3).
- When you can't handle the input, return the **noun form**
  `Expr::call("my_fn", args.to_vec())` — never panic, never guess.

---

## 3. Working with `Expr`

### 3.1 Constructors (re-exported via `maxima_plugin`)

```rust
Expr::int(42)                       // integer
Expr::Float(3.14)                   // float
Expr::Rational { num: 1, den: 2 }   // 1/2  (both i64)
Expr::BigInt(Box::new(big))         // arbitrary-precision integer
Expr::sym("x")                      // symbol
Expr::add(a, b)  Expr::sub(a, b)
Expr::mul(a, b)  Expr::div(a, b)
Expr::pow(base, exp)  Expr::neg(a)
Expr::call("sin", vec![x])          // function application / noun form
```

For a sum or product of many terms, build the list directly:

```rust
use maxima_plugin::Operator;
Expr::List { op: Operator::MPlus, simplified: false, args: terms }
```

### 3.2 Inspecting arguments

```rust
match &args[0] {
    Expr::Integer(n)              => /* exact integer */,
    Expr::Rational { num, den }   => /* exact rational */,
    Expr::Float(x)               => /* numeric */,
    Expr::BigInt(b)              => /* big integer */,
    Expr::Symbol(id)             => /* a symbol */,
    _                            => /* fall back to noun form */,
}
```

A common pattern — accept any numeric, extract `f64`:

```rust
fn as_f64(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        _ => None,
    }
}
```

### 3.3 Simplifying / evaluating a result

Two host functions are re-exported:

- `simplify(&Expr) -> Expr` — algebraic canonicalization (collect like terms,
  power/rational folding). Pure, no `env`.
- `meval(&Expr, &mut Environment) -> Expr` — full evaluation (function
  dispatch + simplification). Use this to fold a polynomial at a numeric point.

Rule of thumb: build the result with the constructors, then call `simplify`
(symbolic) or `meval` (when a numeric argument should collapse to a number).
See `plugins/orthopoly` — it builds a polynomial from exact coefficients and
calls `meval` so `legendre_p(2, 1/2)` folds to `-1/8` while
`legendre_p(2, x)` stays symbolic.

---

## 4. Panic safety (mandatory)

A panic must **never** unwind out of a plugin. The host and the plugin are
separately compiled and have independent panic runtimes; an escaping panic is
a "foreign exception" the host cannot catch, and it **aborts the whole
process**.

Wrap every function body in `guard`, which catches panics inside the plugin's
own runtime and returns the noun form:

```rust
fn my_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("my_fn", args, || {
        // even if this panics (e.g. an out-of-range index, a division that
        // overflows in debug), the host stays alive and gets my_fn(args...)
    })
}
```

The host keeps a `catch_unwind` of its own, but it only protects
statically-linked native functions — it cannot save a dynamically loaded
plugin that lets a panic escape. `guard` is your responsibility.

---

## 5. The symbol interner

Maxima represents identifiers as interned `SymbolId`s. Because a plugin is a
separate dynamic object, it has its **own copy** of the interner's global
state. If left unshared, the same name would map to different ids in the host
and the plugin, so:

- function names wouldn't dispatch, and
- any `Expr` the plugin returns containing symbols would be mis-displayed.

The loader fixes this: at load time it calls the plugin's
`maxima_plugin_set_interner` (generated by the macro), handing over the host's
table. After that, interning is shared and symbols round-trip correctly in
both directions — arguments coming in and results going out.

You get this for free by using `maxima_plugin!`. If you ever hand-write the
exports, you must implement the interner hook or symbolic I/O will be garbled.

---

## 6. Building, loading, and search paths

```sh
cargo build -p maxima-myplugin            # debug:  target/debug/lib...
cargo build -p maxima-myplugin --release  # release: target/release/lib...
```

`load_plugin(name)` resolves `name` in this order:

1. As given (absolute, or `./` / `../` relative), with the platform extension
   (`.so`/`.dylib`/`.dll`) appended if you omit it.
2. Each directory in the `MAXIMA_PLUGIN_PATH` environment variable
   (`:`-separated).
3. Each directory in the session's search paths.

So all of these work:

```maxima
load_plugin("target/debug/libmaxima_myplugin");   /* explicit */
load_plugin("libmaxima_myplugin");                 /* via MAXIMA_PLUGIN_PATH */
load_plugin("libmaxima_myplugin.so");              /* with extension */
```

- `load_plugin` returns `true` on success, or `false` with a message on
  stderr (file not found, missing symbol, or ABI mismatch).
- It is **idempotent**: loading the same resolved path twice is a no-op.
- `loaded_plugins()` returns the list of loaded plugin paths.
- Plugin functions have the highest dispatch priority and **survive
  `kill(all)`**, like built-ins.

---

## 7. ABI compatibility — the one hard rule

The plugin boundary uses the **Rust ABI**, which is *not* stable across
compiler versions or crate versions. Therefore:

> **Build your plugin from this workspace, with the same toolchain as the
> host.**

The host embeds an ABI string (plugin-API revision + crate version + `rustc`
version) and compares it to the plugin's. On mismatch, `load_plugin` refuses
to call the plugin and returns `false` — turning undefined behavior into a
clear error. A source-compatible change at the *same* version is **not**
caught, so always rebuild plugins after changing the kernel.

If you need plugins that survive toolchain upgrades or come from third
parties, that requires a C-ABI or WASM boundary — out of scope for the
current design.

---

## 8. Limitations to design around

- **No big rationals.** `Expr::Rational` is `i64/i64`. Integers promote to
  `Expr::BigInt`, but a rational whose numerator or denominator exceeds `i64`
  cannot be represented exactly; high-degree/large numeric results may stay
  partly unsimplified. Symbolic results are unaffected. (Compute exact
  coefficients with `num::BigRational` internally; emit big integers via
  `Expr::BigInt` and rationals as `Expr::div(BigInt, BigInt)`.)
- **No unloading.** Loaded libraries are kept alive for the session; there is
  no `unload_plugin` (function pointers would dangle).
- **Cap your work.** Give every loop/series a fixed iteration bound and bail to
  the noun form rather than hang on pathological input.

---

## 9. Testing a plugin

Put integration tests under `crates/eval/tests/`. Locate (and build if
necessary) the cdylib, load it into a fresh `Environment`, and assert results.
This pattern is used by `plugin_test.rs`, `orthopoly_test.rs`,
`specfun_test.rs`:

```rust
use maxima_eval::{eval_str_with_env, Environment};
use std::{path::PathBuf, process::Command};

fn artifact(pkg: &str, lib: &str) -> Option<String> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let name = format!("{}{}.{}", std::env::consts::DLL_PREFIX, lib, std::env::consts::DLL_EXTENSION);
    let find = || ["debug", "release"].iter()
        .map(|p| target.join(p).join(&name))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string());
    find().or_else(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let _ = Command::new(cargo).args(["build", "-p", pkg]).status();
        find()
    })
}

#[test]
fn my_plugin_works() {
    let Some(path) = artifact("maxima-myplugin", "maxima_myplugin") else { return; };
    let mut env = Environment::new();
    assert_eq!(eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env), "true");
    assert_eq!(eval_str_with_env("plugin_double(21);", &mut env), "42");
}
```

**Numerical verification is mandatory** for any function implementing a
mathematical formula: assert results against independent reference values to
~1e-9 (parse the float out of the result string and compare). See
`specfun_test.rs`. A formula that passes structural tests but is numerically
wrong is worse than not shipping it.

---

## 10. SDK reference (`maxima_plugin::`)

| Item | Description |
|------|-------------|
| `maxima_plugin!(register = \|env\| { ... })` | Generates the three ABI exports. |
| `guard(name, args, body)` | Run `body`, containing any panic; returns noun on panic. |
| `Expr`, `Operator` | Re-exported expression types. |
| `Environment` | The evaluator environment (`register_native`, variable lookup, ...). |
| `NativeFn` | `fn(&[Expr], &mut Environment) -> Expr`. |
| `simplify(&Expr) -> Expr` | Algebraic simplification. |
| `meval(&Expr, &mut Environment) -> Expr` | Full evaluation. |
| `maxima_core`, `maxima_eval` | The re-exported crates, for anything else. |

---

## 11. Worked examples in the tree

- **`plugins/template`** — minimal starting point.
- **`plugins/orthopoly`** — symbolic results: orthogonal polynomials from
  exact `BigRational` recurrences, emitted as polynomials and folded by the
  host. Shows symbolic + numeric output and the symbol round-trip.
- **`plugins/specfun`** — numeric results: gamma/erf/Bessel via Lanczos,
  series, and continued fractions, each reference-verified. Shows exact
  special cases alongside numeric evaluation.
- **`plugins/test_plugin`, `plugins/bad_plugin`** — fixtures used by the
  loader's own tests (panic containment and the missing-symbol error path).
