# Maxima Kernel Rust Rewrite — Project Overview

## Goal

Rewrite the Maxima CAS kernel from Common Lisp to Rust, preserving
correctness and the existing Maxima-language interface. The rewrite covers
the core evaluation pipeline (parser, evaluator, simplifier, assumption
database, display) and foundational math modules (arithmetic, polynomials,
basic solving). Specialized modules (integration, limits, special functions)
and the `share/` library remain in Maxima-language files loaded at runtime.

## Scope

### In scope (the "kernel")

| Layer | Lisp sources | Approx lines |
|-------|-------------|-------------|
| Expression representation | maxmac, mormac, clmacs | ~2,500 |
| Parser | nparse.lisp | ~1,900 |
| Evaluator | mlisp.lisp, mmacro.lisp, buildq.lisp | ~2,700 |
| Simplifier core | simp.lisp | ~3,300 |
| Comparison / assumptions | compar.lisp, askp.lisp | ~2,900 |
| Arithmetic / numbers | float.lisp, numeric.lisp, rat3*.lisp | ~6,500 |
| Polynomial / rational | factor.lisp, rat3a-e, result.lisp | ~5,000 |
| Display | displa.lisp, grind.lisp, mactex.lisp | ~4,100 |
| REPL / system | macsys.lisp, init-cl.lisp, mload.lisp | ~3,300 |
| Error handling | merror.lisp | ~500 |
| Globals / config | globals.lisp | ~1,900 |
| **Total** | | **~34,600** |

### Out of scope (kept as `.mac` or future sprints)

- Symbolic integration (defint, risch, antid — ~8,000 lines)
- Limits (limit, tlimit — ~4,600 lines)
- Special functions (gamma, bessel, ellipt — ~31,000 lines)
- Numerical SLATEC/QUADPACK routines (~28,500 lines)
- `share/` packages (lapack, cobyla, draw, tensor, etc.)
- GUI frontends (xmaxima, emacs modes)
- Plot subsystem (plot.lisp — ~2,700 lines)

Out-of-scope Maxima-language files (`.mac`) will be loaded by the new
Rust kernel's file-loader, preserving backward compatibility.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                    Rust Kernel                        │
│                                                      │
│  ┌─────────┐  ┌───────────┐  ┌────────────────────┐ │
│  │ Parser  │→│ Evaluator  │→│ Simplifier         │ │
│  │ (nparse)│  │ (meval)    │  │ (simp + operators) │ │
│  └─────────┘  └───────────┘  └────────────────────┘ │
│       ↑              ↓               ↓               │
│  ┌─────────┐  ┌───────────┐  ┌────────────────────┐ │
│  │ Display │  │ Assumption│  │ Rational / Poly    │ │
│  │ (displa)│  │ Database  │  │ Arithmetic         │ │
│  └─────────┘  └───────────┘  └────────────────────┘ │
│                      ↓                               │
│              ┌───────────────┐                       │
│              │ .mac Loader   │ ← loads share/, tests │
│              └───────────────┘                       │
└──────────────────────────────────────────────────────┘
```

## Release Cadence

| Release | Content | Verifiable milestone |
|---------|---------|---------------------|
| **RC0** | Project skeleton, expression types, basic REPL | `1+1` evaluates to `2` |
| **RC1** | Parser + evaluator for core Maxima syntax | `rtest1.mac` passes |
| **RC2** | Simplifier + polynomial arithmetic | `rtest1.mac`–`rtest4.mac` pass |
| **RC3** | Assumption database + comparison | `rtest_ask1.mac`, `rtest_boolean.mac` pass |
| **RC4** | Rational functions + factoring | `rtest5.mac`–`rtest8.mac` pass |
| **RC5** | File loading + `.mac` compatibility | `rtest9.mac`–`rtest11.mac`, `share/` loads |
| **RC6** | Display (2D, TeX) + full test suite | 60+ rtest files pass |

Each RC maps to 2–4 sprints. See individual sprint files for details.
