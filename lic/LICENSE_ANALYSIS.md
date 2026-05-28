# License Analysis — Maxima Kernel (Rust)

## Current License

The Rust kernel is licensed as **GPL-2.0-only**, matching the original
Maxima (Common Lisp) project.

## Dependency License Summary

| License | Count | Dependencies |
|---------|-------|-------------|
| MIT OR Apache-2.0 | 22 | num, num-bigint, libc, cfg-if, log, tempfile, syn, proc-macro2, ... |
| MIT | 9 | plotters, plotters-backend, plotters-svg, rustyline, nix, radix_trie, ... |
| Apache-2.0 OR MIT | 3 | autocfg, fastrand, utf8parse |
| Apache-2.0 WITH LLVM-exception | 2 | rustix, linux-raw-sys |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | unicode-ident |
| Unlicense OR MIT | 1 | memchr |
| **GPL-2.0-only** | **5** | **maxima-core, maxima-eval, maxima-parser, maxima-poly, maxima-repl** |

## Analysis

### Can we switch to Apache-2.0/MIT?

**Yes, in principle.** The Rust kernel is a ground-up reimplementation — no
code was copied from the original Maxima Lisp codebase. All algorithms were
reimplemented from mathematical descriptions, not from Maxima's source code.

**All dependencies are permissively licensed** (MIT, Apache-2.0, or dual).
None are GPL/LGPL. There is no GPL dependency forcing copyleft.

The GPL-2.0-only license was chosen to match the original Maxima project
by convention, not by legal necessity.

### Switching considerations

| Factor | Assessment |
|--------|-----------|
| Dependencies | All MIT/Apache-2.0 — no blockers |
| Original Maxima code reuse | None — clean-room reimplementation |
| Algorithm descriptions | Mathematical algorithms are not copyrightable |
| gnuplot | We generate scripts only, no linking — no license issue |
| plotters crate | MIT — compatible with any license |
| CLAUDE.md says GPL-2.0 | Can be updated if decision changes |

### gnuplot Compatibility

The `gnuplot_script()` function **generates a text file** containing
gnuplot commands. It does NOT link against gnuplot, does NOT include
gnuplot code, and does NOT depend on gnuplot being installed. The script
file is a standalone artifact the user can optionally run with gnuplot.

gnuplot itself is not a dependency — it's an optional external tool.
This is analogous to generating LaTeX output: we produce `.tex` but
don't depend on a TeX distribution.

The `plot2d()` function uses the **plotters** crate (MIT license) to
generate SVG directly — no external tools needed.

### Recommendation

The kernel CAN be licensed under MIT or Apache-2.0 or dual MIT/Apache-2.0
if desired. The only thing tying it to GPL is the project convention.

To switch:
1. Change `license = "GPL-2.0-only"` to `license = "MIT OR Apache-2.0"` in `Cargo.toml`
2. Add LICENSE-MIT and LICENSE-APACHE files
3. Update CLAUDE.md and README.md references

## Dependency Details

```
PERMISSIVE (all dependencies):
  plotters 0.3.7              MIT
  plotters-backend 0.3.7      MIT
  plotters-svg 0.3.7          MIT
  rustyline 15.0.0            MIT
  rustyline-derive 0.11.1     MIT
  nix 0.29.0                  MIT
  radix_trie 0.2.1            MIT
  num 0.4.3                   MIT OR Apache-2.0
  num-bigint 0.4.6            MIT OR Apache-2.0
  num-traits 0.2.19           MIT OR Apache-2.0
  libc 0.2.186                MIT OR Apache-2.0
  tempfile 3.27.0             MIT OR Apache-2.0
  memchr 2.8.0                Unlicense OR MIT
  (all others)                MIT OR Apache-2.0

GPL/LGPL/copyleft: NONE
```
