# v4.0 Sprint Index

## Status: ✅ Complete (2026-05-26)

All sprints through V5.1 complete. 822 tests, zero failures. Code reviewed.

## Sprints

| Sprint | Content | Status |
|--------|---------|--------|
| **V4.1** | CLI + script runner (5 modes, --help, stdin pipe) | ✅ Done |
| **V4.2** | PolyAlg — Q(α)[x] arithmetic with GCD | ✅ Done |
| **V4.3** | Algebraic factoring → ∫ 1/(x⁴+1) solved | ✅ Done |
| **V4.3+** | Trager norm shift, cyclotomic Q(ω), i128 overflow protection | ✅ Done |
| **V4.4** | Radical patterns: √x·log(x), x²/√(x²+c), completing square | ✅ Done |
| **V4.4+** | Euler substitution for ∫ 1/((x+a)√(x²+c)), √(x²+c)/x² | ✅ Done |
| **V4.5** | Gamma integral: ∫₀^∞ x^n·exp(-x) = n! | ✅ Done |
| **V4.5+** | Laplace transforms, Gaussian-cosine ∫ exp(-ax²)cos(bx) | ✅ Done |
| **V4.6** | File I/O: save, stringout, printfile, file_search | ✅ Done |
| **V4.7** | Benchmark suite (70+ checks) + 4 bug fixes | ✅ Done |
| **V5.0** | Plugin-ready package system: NativeFn, autoload, require | ✅ Done |
| **V5.1** | Bug fixes (11), REPL tab completion, user manual, 3 walkthroughs | ✅ Done |

## Key Results

```
$ maxima-repl -e "integrate(1/(x^4+1), x);"
log+atan with √2 coefficients

$ maxima-repl -e "integrate(x^n*exp(-x), x, 0, inf);"
factorial(n)

$ maxima-repl -e "integrate(1/((x+1)*sqrt(x^2+5)), x);"
log(abs((sqrt(x^2+5)-2)/(x+1)))/2

$ maxima-repl -e "integrate(exp(-2*x^2)*cos(3*x), x, 0, inf);"
sqrt(pi/2)/2 * exp(-9/8)

$ maxima-repl -e "solve(a*x^2+b*x+c=0, x);"
[x = (-b+√(b²-4ac))/(2a), x = (-b-√(b²-4ac))/(2a)]
```

## Future Work (not implemented)

| Item | Description | See |
|------|-------------|-----|
| Rust plugin loading | Dynamic `.so`/`.dylib` loading via `dlopen` | Plugin API in README |
| General Euler substitution | ∫ R(x, √(ax²+bx+c)) for arbitrary R | `requestFeature/radical-risch.md` |
| Full Almkvist-Zeilberger | Parametrized continuous Gosper with Risch DE | `requestFeature/Almkvist-Zeilberger.md` |
| Output redirection | `writefile()` / `closefile()` | — |
| Hash-consed DAG | O(1) equality via arena allocation | `requestFeature/hash-consed-dag.md` |
| Matrix arithmetic | Element-wise `+`, `-`, `*`, dot product `.` | — |
| Batch mode re-parse | Fix Display→re-parse round-trip for if/for/block | — |

## Documents

| File | Contents |
|------|----------|
| [PLAN.md](PLAN.md) | Full sprint plan with task checklists |
