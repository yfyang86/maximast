# RC5 — File Loading + `.mac` Compatibility

**Goal:** Load and execute Maxima-language files (`.mac`, `.lisp` stubs),
enabling the `share/` library ecosystem. Pass `rtest9–rtest11`.

**Exit criteria:** `load("share/contrib/some_file.mac")` works;
`batch("tests/rtest9.mac")` runs; `.mac` test files load and execute.

---

## Sprint 5.1 — File Loader

**Duration:** 2 weeks

### Tasks

- [ ] `load(filename)` — find and execute a `.mac` file:
  - Search path: current dir, `share/`, `tests/`, configured paths
  - `file_search_maxima` — list of search directories
  - `file_search_lisp` — for Lisp compatibility stubs
  - File extension handling: `.mac`, `.mc`, `.lisp`
- [ ] `batch(filename)` — load file in batch mode (like `load` but with
  output suppression control)
- [ ] `batchload(filename)` — load without echoing
- [ ] `file_search(name, paths)` — search for file in path list
- [ ] `pathname_directory`, `pathname_name`, `pathname_type` — path utilities
- [ ] `stringout(filename, exprs...)` — write expressions to file

### Tests

```
#[test] fn load_mac_file()  { run("load(\"tests/rtest1.mac\");") succeeds }
#[test] fn batch_file()     { run("batch(\"tests/rtest1.mac\", test);") runs tests }
#[test] fn file_search()    { run("file_search(\"rtest1.mac\");") finds file }
```

---

## Sprint 5.2 — Package System

**Duration:** 2 weeks

### Tasks

- [ ] `setup_autoload(file, fn1, fn2, ...)` — register functions for autoloading
- [ ] Autoload mechanism: on first call to unresolved function, check autoload
  table, load file, retry
- [ ] `packagefile: true/false` — flag to suppress `values`/`functions` pollution
- [ ] `save(filename, vars...)` — save variable values to file
- [ ] `restore(filename)` — load saved values
- [ ] `loadfile(filename)` — load a save file
- [ ] Handle `share/` package loading conventions:
  - `load("draw")` → finds `share/draw/draw.mac`
  - `load("descriptive")` → finds appropriate file

### Tests

```
#[test] fn autoload_basic() { run("setup_autoload(\"test_pkg.mac\", foo); foo(1);") == ... }
#[test] fn packagefile()    { run("packagefile:true; load(\"pkg.mac\"); functions;") excludes pkg fns }
```

---

## Sprint 5.3 — I/O Functions

**Duration:** 2 weeks

### Tasks

- [ ] String operations:
  - `concat(a, b, ...)` — concatenate
  - `sconcat(a, b, ...)` — concatenate to string
  - `string(expr)` — convert expression to string
  - `parse_string(str)` — parse string as Maxima expression
  - `eval_string(str)` — parse and evaluate
  - `substring(str, start, end)`
  - `slength(str)` — string length
  - `charat(str, n)` — character at position
  - `ssearch(target, str)` — search for substring
  - `split(str, delim)` — split string
- [ ] Stream I/O:
  - `openr(file)`, `openw(file)`, `opena(file)` — open file
  - `close(stream)` — close
  - `readline(stream)` — read line
  - `readchar(stream)` — read character
  - `printf(stream, fmt, args...)` — formatted output
  - `sprint(expr)` — print expression to string
- [ ] `with_stdout(file, expr)` — redirect output to file

### Tests

```
#[test] fn concat_basic()      { run("concat(\"hello\", \" \", \"world\");") == "hello world" }
#[test] fn parse_string()      { run("parse_string(\"x+1\");") == "x+1" }
#[test] fn eval_string()       { run("eval_string(\"1+2\");") == "3" }
#[test] fn string_ops()        { run("slength(\"abc\");") == "3" }
```

---

## Sprint 5.4 — rtest9–rtest11 + Matrix Basics

**Duration:** 3 weeks

### Tasks

- [ ] Matrix operations (needed for rtests):
  - `matrix([row1], [row2], ...)` — construct matrix
  - `determinant(M)`, `invert(M)`, `transpose(M)`
  - `eigenvalues(M)`, `eigenvectors(M)` — basic cases
  - `rank(M)`
  - Matrix arithmetic: `M1 + M2`, `M1 . M2` (dot product)
  - `ident(n)` — identity matrix
  - `zeromatrix(m, n)` — zero matrix
  - `diagmatrix(n, val)` — diagonal matrix
  - `row(M, i)`, `col(M, j)` — extract row/column
- [ ] Dot operator `.` for non-commutative multiplication
- [ ] `map(fn, list)` on matrices (apply to each element)
- [ ] Run `rtest9.mac`–`rtest11.mac`, fix failures iteratively

### Tests

```
#[test] fn matrix_det()       { run("determinant(matrix([1,2],[3,4]));") == "-2" }
#[test] fn matrix_inverse()   { run("invert(matrix([1,2],[3,4]));") == correct }
#[test] fn matrix_mul()       { run("matrix([1,2],[3,4]) . matrix([5],[6]);") == "matrix([17],[39])" }
#[test] fn rtest9()           { assert_rtest_passes("tests/rtest9.mac"); }
#[test] fn rtest10()          { assert_rtest_passes("tests/rtest10.mac"); }
#[test] fn rtest11()          { assert_rtest_passes("tests/rtest11.mac"); }
```

---

## Deliverable

```
$ maxima-kernel
(%i1) load("share/linearalgebra/linearalgebra.mac");
(%o1)             share/linearalgebra/linearalgebra.mac
(%i2) M : matrix([1,2,3],[4,5,6],[7,8,10]);
                          [ 1  2  3  ]
(%o2)                     [ 4  5  6  ]
                          [ 7  8  10 ]
(%i3) determinant(M);
(%o3)                         -3
(%i4) batch("tests/rtest9.mac", test);
...all tests pass...
```
