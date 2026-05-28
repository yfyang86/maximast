# A Symbolic Computation Algorithm Survey for a Rust Refactor of Maxima

## TL;DR
- **Wrap, don't reinvent, the polynomial/number-theoretic core.** FLINT 3 (LGPL v3+) via `flint3-sys` and msolve (GPLv2+) cover Gröbner, factorization, GCD, ball arithmetic, real-root isolation, and algebraic numbers at world-class performance; nothing native to Rust currently beats them, and Symbolica is source-available/non-redistributable so it cannot be a dependency.
- **Implement the differential-algebra layer (Risch–Bronstein–Trager, Gruntz limits, Karr/Schneider summation, Almkvist–Zeilberger, holonomic closure) natively in Rust.** This is where Maxima is structurally weakest, where FriCAS leads, and where there is no Rust competitor. The work is ~25–40 person-months across families and is the strategic differentiator.
- **GPL is the single biggest licensing hazard.** Maxima itself is GPLv2+; msolve, Singular, and PARI/GP are GPL; FLINT/Arb/GMP/MPFR/NTL are LGPL. If the refactor wants a permissive (MIT/Apache) license, the polynomial-system solver path goes through writing an F4/F5 in Rust (Groebner.jl-style) or a clean-room reimplementation, not msolve FFI.

---

## Summary Table

| Family | Recommendation | Primary reference to study | Effort (PM) |
|---|---|---|---|
| Rational-function integration (Hermite, Lazard–Rioboo–Trager) | **GO (native)** | Bronstein, *Symbolic Integration I*, ch. 2 | 2–3 |
| Risch transcendental (Bronstein 1990/2005) | **GO (native)** | FriCAS `intef.spad`, `intrf.spad`; Bronstein book | 8–14 |
| Risch algebraic (Trager 1984; Bronstein ext.; Kauers heuristic) | **GO (native, partial)** | Trager thesis; FriCAS `algfunc/algint`; Sutherland 2017 | 6–10 |
| Risch–Norman / parallel-Risch heuristic | **GO (native)** | Boettner 2010 thesis; Geddes/Czapor/Labahn ch. 12 | 2 |
| Parametric / Liouvillian (Raab) | **DEFER** to v2 | Raab JKU PhD 2012 | 4–6 |
| Definite int.: Almkvist–Zeilberger, Meijer-G | **GO (AZ native)**, **DEFER (Meijer-G)** | Koutschan *HolonomicFunctions*; Adamchik–Marichev | 4 / 8 |
| Limits — Gruntz / MrvAsympt | **GO (native)** | Gruntz PhD 1996; SymPy `gruntz.py` | 1.5 |
| Transseries / D-finite asymptotics | **DEFER** | van der Hoeven, *Transseries and RDA* | 6+ |
| Gosper hypergeometric summation | **GO (native)** | *A=B* (Petkovšek/Wilf/Zeilberger) | 0.5 |
| Zeilberger / WZ creative telescoping | **GO (native)** | Koutschan 2010, 2013 surveys | 2 |
| Karr / Schneider ΠΣ summation | **GO (native, partial)** | Schneider, *J. Symb. Comp.* 72 (2016) | 6–10 |
| Holonomic / D-finite closure | **GO (native)** | Koutschan *HolonomicFunctions* user's guide | 3–5 |
| Gröbner bases (F4, multi-modular FGLM) | **GO via FFI (msolve, GPL)** OR **GO (native, MIT path)** | Berthomieu/Eder/Safey El Din ISSAC '21; Groebner.jl | 1 (FFI) / 8–12 (native) |
| Signature-based F5 / GVW / M5GB | **DEFER** | Eder–Faugère survey; M5GB 2022 | 4 |
| Triangular decomposition / regular chains | **DEFER** | Chen–Moreno Maza JSC 47 (2012) | 6 |
| Resultants / subresultants / Bezoutians | **GO (FFI to FLINT)** | FLINT `fmpz_poly`, `nmod_poly` | 0.5 |
| CAD (Brown–McCallum, Lazard 2020) | **DEFER (v2)** | Brown–McCallum CASC 2020 | 8–12 |
| Real root isolation (VAS/Descartes) | **GO via FFI (FLINT)** + native fallback | FLINT `fmpz_poly_roots`; Akritas–Strzeboński | 1 (FFI) |
| Polynomial GCD (Brown, Zippel, Hu–Monagan, Huang–Monagan 2024) | **GO (FFI for v1; native sparse later)** | Hu–Monagan JSC 105 (2021); ISSAC '24 | 0.5 / 4 |
| Polynomial factorization (BZ, van Hoeij) | **GO via FFI (FLINT)** | van Hoeij–Novocin 2010 | 0.5 |
| RootOf / algebraic-number arithmetic | **GO via FFI (FLINT `ca`, `fq`)** | Calcium docs; Trager 1984 | 1 |
| Algebraic simplification with side relations | **GO (native, on Gröbner)** | Geddes/Czapor/Labahn ch. 10 | 1 |
| ODE/PDE solvers (Kovacic, Prelle–Singer, Lie) | **NO-GO for v1** | — | — |

---

## Key Findings

1. The dividing line between FFI and native code in this project is the *algebra/algebraic-geometry layer* (FFI to FLINT, optionally msolve) versus the *differential/difference-algebra layer* (native Rust). The first is mature, optimized, and not worth re-implementing; the second is where research-level work continues and where Maxima needs the most upgrade.
2. Among open systems, **FriCAS is the technically strongest reference for the integration tower**, both algorithmically (Bronstein authored its integrator) and structurally (its SPAD category/domain hierarchy). The hierarchy — not the algorithms per se — is what Maxima missed.
3. **No serious Rust CAS exists today.** Symbolica is fast and well-engineered but is source-available/commercial. Other Rust crates (`cas-rs`, `mathcore`, `rusymbols`, `symbolic_math`) are toys. `mathru` is numerical only. The opportunity is real.
4. **License posture is the most important early decision.** Maxima is GPLv2+; if the refactor inherits that, msolve/Singular/PARI are usable, dramatically reducing the polynomial-system effort. If the refactor wants MIT/Apache (e.g. for industrial adoption), every GPL dependency must be excised, with corresponding cost.

---

## 1. Symbolic Integration

### 1.1 Rational-function integration
The base case — integration of `p(x)/q(x)` in `Q(x)` — is solved by Hermite reduction plus the Lazard–Rioboo–Trager algorithm for the logarithmic part (Lazard–Rioboo, *J. Symb. Comp.* 9, 1990, 113–115). It is fully algorithmic, complexity-polynomial, and a prerequisite for everything else. There is no Rust implementation; FLINT exposes the necessary subresultant/PRS machinery (`fmpz_poly_resultant`, `fmpq_poly`). **GO native (≈2–3 PM)**: this is the spine of the integration tower; you want it as native Rust code over your own polynomial type (with FLINT as fallback for big examples).

### 1.2 Risch — transcendental case (Bronstein's complete treatment)
Bronstein's 2005 book *Symbolic Integration I — Transcendental Functions* (2nd ed., Springer) is the canonical algorithmic reference. The full recursive scheme — primitive case, exponential case, parametric Risch differential equation, structure theorems, denominator bounds, polynomial-part reduction — is the dominant cost center for a CAS integrator. **The current state of the art is FriCAS**: per the FriCAS wiki (`SymbolicIntegration`), "FriCAS implementation of Risch algorithm is probably the 'most complete' existing implementation"; per the FriCAS features page (fricas.org/features.html), FriCAS offers "integration (most complete implementation of the Risch algorithm)." The transcendental core was rewritten in 2014 to eliminate all known reasons for incompleteness (per FriCAS wiki page `RischImplementationStatus`). SymPy's `risch.py` implements a fragment; Maple and Mathematica fall back on Risch–Norman heuristic + table-based rules (RUBI); Maxima ships Moses's 1967 SIN plus a partial Risch.

Recent work worth tracking:
- C. G. Raab, *Definite Integration in Differential Fields* (JKU PhD 2012), extends parametric Risch to Liouvillian extensions, giving a decision procedure where Bronstein only had partial results.
- W. Hebisch, "Symbolic integration in the spirit of Liouville, Abel and Lie" (arXiv:2104.06226, 2021) — Liouville principle for elliptic integrals, basis for future FriCAS extensions.
- C. G. Raab, "Comments on Risch's *On the Integration of Elementary Functions which are Built Up Using Algebraic Operations*" (2022, in *Anti-Differentiation and the Calculation of Feynman Amplitudes*, Springer) — important commentary on gaps.
- **`SymbolicIntegration.jl` v3.1.0** by Harald Hofstätter, Mattia Micheletta Merlin, and Chris Rackauckas (SciML, announced October 10, 2025 at sciml.ai/news/2025/10/10/SymbolicIntegration/) is a Julia re-implementation following Bronstein's book. The blog explicitly states "the Risch algorithm is theoretically complete for elementary functions, [but] it struggles with algebraic functions like sqrt(x) and non-integer powers"; algebraic-function support is marked ❌ in the package's Risch feature-comparison table.

**No Rust crate implements Risch.** Symbolica is a fast polynomial/expression engine but has no Risch; `mathcore`, `rusymbols`, `cas-rs` offer only toy symbolic differentiation/heuristic integration.

**GO native, 8–14 PM.** Implement the transcendental tower (primitive, exp, log; Risch d.e.; integer/rational denominators; Rothstein–Trager–Lazard–Rioboo logarithmic part) on top of your own differential-field abstraction. Use FLINT's `fmpq_poly`/`fmpz_poly` underneath. The biggest implementation traps: (i) the "structure theorems" (Bronstein 2007, *J. Symb. Comp.* 42, 757–769) for deciding algebraic dependencies among monomials in a tower; (ii) the parametric logarithmic-derivative problem; (iii) zero-test in the constant field, which is undecidable in general (this is why Risch is a *semi-algorithm*).

### 1.3 Risch — algebraic case (Trager, Bronstein, Kauers, Sutherland)
This is the hardest part of indefinite integration. Trager's 1984 MIT thesis is the foundation; Bronstein (1990, *J. Symb. Comp.* 9) extended it to elementary functions over algebraic extensions; Sutherland's "Trager's Algorithm for Integration of Algebraic Functions Revisited" (PSU, 2016) fills in gaps and partially implements in Mathematica; Kauers (ISSAC '08) gives a useful heuristic for the logarithmic part of algebraic integrals.

Per Sutherland: "Building on work of Risch in the 1980s and Liouville in the 1840s, Trager published an algorithm for deciding if a given algebraic function has an elementary antiderivative. While this algorithm is theoretically complete, it is incomplete in the sense that assumptions are made about the function to be integrated in relation to the defining equation for the algebraic irrationality." FriCAS's `intalg.spad` lists four explicit cases of "implementation incomplete": constant residues, irrational residues, certain non-radical extensions, and mixed transcendental-algebraic towers. A benchmark from arXiv:2004.04910 reports on a pseudo-elliptic test suite: FriCAS solves 75.4%, AXIOM 48.7%, Maple 11.6%, Mathematica 9.5%, REDUCE/algint 6.3%, Rubi 13.7% — i.e. FriCAS is roughly an order of magnitude ahead of the closed-source competition on algebraic integrands.

Recent work to follow: Trager's own survey "Comments on Integration of Algebraic Functions" (2022, Springer TMSC); Chen–van Hoeij–Kauers–Koutschan, "Reduction-based creative telescoping for Fuchsian D-finite functions" (*J. Symb. Comp.* 85, 2018, 108–127), provides an alternative reduction path for definite algebraic integrals.

**GO native (partial), 6–10 PM.** Implement Trager's "rational" subset (radical-only extensions) first; this is what handles `int(p(x,sqrt(...)), x)` for most user inputs. Punt the general "non-radical" case to v2.

### 1.4 Risch–Norman / parallel Risch heuristic
The Risch–Norman heuristic (Davenport, EUROCAM 1982, then refined; Boettner's 2010 Tulane PhD covers mixed transcendental-algebraic) makes an ansatz for the integral, differentiates, and solves a linear system. It is fast, often works, and is not a decision procedure — so it is the right *first try* before the full Risch decision procedure. Maple's default `int` is essentially this. **GO native, ~2 PM.** Cheap to implement, big quality-of-life win.

### 1.5 Definite integration: Almkvist–Zeilberger, Meijer-G
- **Almkvist–Zeilberger** (1990) is creative telescoping for hyperexponential integrands; it is essentially Zeilberger's algorithm transposed to `∂x` and gives a linear differential equation satisfied by the parameter integral. Implementation is a few hundred lines on top of Ore-algebra arithmetic. **GO native, ~4 PM.**
- **Meijer G-function methods** are how Mathematica's `Integrate` handles most "named" definite integrals (Adamchik–Marichev approach: match against a G-function table, look up the answer using the Slater theorem, simplify). Building a Meijer-G table and the matching engine is large (Maple's and Mathematica's tables are decades of work). The SymPy implementation (`sympy/integrals/meijerint.py`) is the only open-source reference. **DEFER, ~8 PM if needed.**

### 1.6 Recent frontier — non-Liouvillian, parametric, special-function
- Raab (2012 thesis; ISSAC 2013 short paper) gives a decision procedure for parametric elementary integration over regular Liouvillian extensions and partial results for integration in terms of error function, exponential integral, polylogarithms.
- Chen–Du–Li, "Additive Decompositions in Primitive Extensions" (ISSAC '18) — modern reductions that unify Hermite reduction across differential structures.
- These are essentially research-grade. **DEFER to v2** unless your user community demands special-function antiderivatives.

---

## 2. Limits

### 2.1 Gruntz / MrvAsympt
Gruntz's 1996 ETH PhD algorithm computes limits at infinity by repeatedly identifying the "most rapidly varying" (mrv) subexpression, rewriting in a uniformization variable `ω`, expanding in series, and recursing on the leading coefficient. SymPy's `sympy/series/gruntz.py` is a direct port of the Maple code from Gruntz's thesis and is the canonical open-source implementation. Known failure modes (per SymPy issue tracker and Gruntz's own discussion):
1. The `rewrite` step is the hard one — bugs typically live in series expansion of logarithmic singularities, not in `mrv` itself.
2. Constant-field zero test is needed (e.g., `lim (sin(x)^2 + cos(x)^2 - 1)` requires recognizing the identity).
3. Multivalued functions on branch cuts (the `log` expansion patch documented in the 2022 GSoC proposal "Improving Series Expansions and Limit Computations").
4. The algorithm assumes all subexpressions are "comparable" — oscillatory `sin(x)` at infinity breaks this and must be special-cased.

**GO native, ~1.5 PM.** Gruntz is a small algorithm (~1k lines in SymPy) but depends on a robust *series expansion engine over your expression type*, which is itself non-trivial.

### 2.2 Asymptotic expansions of D-finite functions and transseries
- Salvy–Shackell "Asymptotic expansions of exp-log functions" tradition: Maple's `MultiSeries` package is the reference.
- van der Hoeven's transseries (PhD 1997, *Transseries and Real Differential Algebra* LNM 1888, 2006) and Aschenbrenner–van den Dries–van der Hoeven *Asymptotic Differential Algebra and Model Theory of Transseries* (Princeton, 2017) provide a theoretical universal domain for these expansions. Per van der Hoeven: "this requires the development and implementation of fast, certified and numerically stable algorithms for multi-precision computations."
- Implementations exist in TeXmacs / `mathemagix` (van der Hoeven's own system) and partially in Maple's `MultiSeries`.

**DEFER, 6+ PM.** This is where the research frontier is for limits at the moment; it would be a major contribution, but Gruntz covers >99% of real user queries.

---

## 3. Symbolic Summation

### 3.1 Gosper's algorithm (hypergeometric)
1978 algorithm; closed-form indefinite hypergeometric summation; about 200 lines of code on top of polynomial GCD. **GO native, 0.5 PM.** No-brainer.

### 3.2 Zeilberger / Wilf–Zeilberger pairs
Creative telescoping for definite hypergeometric sums; produces a linear recurrence satisfied by the sum. The fastest classical implementations are Zeilberger's own `EKHAD` (Mathematica) and Koutschan's `HolonomicFunctions` package (Mathematica). Per Koutschan (arXiv:1307.4554), "creative telescoping is a widely used paradigm in computer algebra, in order to treat symbolic sums and integrals in an algorithmic way. Its modus operandi is to derive, from an implicit description of the summand resp. integrand, e.g., in terms of recurrences or differential equations, an implicit description for the sum resp. integral." **GO native, ~2 PM.**

### 3.3 Karr's algorithm and Schneider's Sigma
Karr (*J. ACM* 28, 1981, 305–350) gave the analogue of the Risch algorithm for indefinite summation in `ΠΣ`-fields (towers of difference-field extensions by sums and products). Schneider (*J. Symb. Comp.* 72, 2016, 82–127, "A difference ring theory for symbolic summation") extends this substantially — the resulting Mathematica package `Sigma` is *the* tool used for Feynman-integral evaluation in particle physics (Ablinger/Blümlein/Schneider/RISC pipeline; see *Computer Algebra in Quantum Field Theory*, Springer 2013).

Sigma is closed-source: the RISC website (risc.jku.at/sw/sigma/, maintained by Carsten Schneider) states, "The source code for this package is password protected. To get the password send an email to Carsten Schneider. It will be given for free to all researchers and non-commercial users." This makes Karr+Schneider one of the highest-value targets for an open implementation. **GO native, 6–10 PM.** Karr alone is ~3 PM (it's structurally parallel to Risch transcendental); Schneider's full ΠΣ* refinement (denominator bounds, depth optimization, telescoper-of-minimal-depth) is the rest.

### 3.4 Koutschan's HolonomicFunctions, Ablinger/Schneider for QFT, q-analogs
- Koutschan (Mathematica, RISC report 10-01, 2010) implements holonomic-closure-properties and creative telescoping for general holonomic functions; converts mathematical expressions to holonomic descriptions automatically. Underpins much QFT computation.
- Ablinger's `HarmonicSums` package handles nested harmonic / cyclotomic / binomial sums for higher-loop master integrals.
- q-Zeilberger (Riese, Paule) is a parallel q-analog tradition; less critical for a general-purpose CAS.

**GO native, 3–5 PM** for the closure-properties + telescoping engine on top of an Ore-algebra polynomial ring. This is also the engine you reuse for definite integration (Almkvist–Zeilberger, Chyzak).

---

## 4. Polynomial Systems

### 4.1 Gröbner bases — Buchberger, F4, F5, GVW, M4GB, M5GB
**State of the art (2024–2026):** `msolve` (Berthomieu–Eder–Safey El Din, ISSAC '21, ACM doi:10.1145/3452143.3465545, pp. 51–58) is the leading open-source library; it implements F4 with multi-modular tracing, sparse-FGLM change of order, and a fast univariate solver. Per the paper, "The cross-over point of our asymptotically fast implementation of the Taylor shift against the classical implementations used in current real root solvers is around degree 512." It outperforms Magma and Maple on rational-coefficient zero-dimensional systems, particularly for real-solution counting and isolation. **License: GPLv2-or-later** (with some files LGPL-2.1+), which is a hard constraint for a permissive Rust port.

Recent work:
- Berthomieu–Neiger–Safey El Din, "Faster Change of Order Algorithm for Gröbner Bases Under Shape and Stability Assumptions" (ISSAC '22) — sparse-FGLM improvements.
- M4GB (Makarim–Stevens, ISSAC '17) — tail-reduced reductors, an order of magnitude memory reduction on dense MQ-challenge problems.
- M5GB (Eder–Pfeiffer 2022) — combines signature-based F5 criteria with M4GB reductors.
- GVW (Gao–Volny–Wang, *Math. Comp.* 85, 2015, 449–465) — the cleanest modern signature framework; the basis for Eder–Faugère's survey "A survey on signature-based Gröbner basis computations."
- `Groebner.jl` (Demin–Gowda, arXiv:2304.06935) — Julia implementation, "the ratio of the runtime of Groebner.jl to the runtime of msolve is within [0.51, 1.42]" — i.e. a careful pure-language F4 can match msolve within 2×, which validates the native-Rust path. Demin's documentation explicitly notes: "In our F4 implementation, we adapt and adjust the code of monomial hashtable, critical pair handling and symbolic preprocessing, and linear algebra from msolve" — pointing to msolve's data structures as the right reference design.

**Two acceptable choices:**
1. **GO via FFI to msolve (1 PM)** if you accept GPL contamination. Easiest.
2. **GO native F4 in Rust (8–12 PM)** if you want to stay permissive. Port Groebner.jl's architecture — monomial hashtable, critical-pair handling, symbolic preprocessing, sparse linear algebra over `Z/pZ` (use SIMD intrinsics for 31-bit prime fields; this is where Rust shines).

Signature-based F5/GVW/M5GB are **DEFER (4 PM)** — they help on structured / overdetermined inputs (cryptanalytic systems) but Berthomieu–Eder–Safey El Din chose plain F4 for msolve precisely because F4 wins on generic dense rational input.

### 4.2 Triangular decomposition / regular chains
Chen–Moreno Maza (*J. Symb. Comp.* 47, 2012, 610–642), Lemaire–Moreno Maza–Xie's `RegularChains` library in Maple (~70 000 lines of C+Maple), Dahan–Schost on sharp estimates. Powerful for parametric/positive-dimensional systems where Gröbner blows up. Open-source equivalents are thin (the Maple library is the reference). **DEFER (~6 PM)** — niche compared to Gröbner; only matters if your user base does parametric algebraic geometry.

### 4.3 Resultants, subresultants, Bezoutians
Classical, fully algorithmic. FLINT covers everything (`fmpz_poly_resultant`, `nmod_poly`, multivariate Sylvester). **GO via FFI to FLINT, 0.5 PM.**

### 4.4 Cylindrical Algebraic Decomposition (CAD)
The current best practical algorithm is the Brown–McCallum improvement of Lazard's projection (Brown–McCallum, CASC 2020, "Enhancements to Lazard's Method for Cylindrical Algebraic Decomposition"): per the abstract, "The present work improves Lazard's method so that it is as efficient for well-oriented input as Brown's method, while retaining its infallibility." Recent threads (CASC 2024 invited talk by Matthew England, arXiv:2407.19781) integrate CAD with SMT solvers and ML-based variable-ordering heuristics; see also Nalbach–Ábrahám–Specht–Brown–Davenport–England, "Levelwise construction of a single cylindrical algebraic cell" (*J. Symb. Comp.* 123, 2024, 102288). The triangular-decomposition-based CAD in `RegularChains` (Chen–Moreno Maza et al.) is a competitive alternative.

Implementations: Mathematica `CylindricalDecomposition` (Strzeboński), QEPCAD-B, Maple `RegularChains[SemiAlgebraicSetTools]`, SMT-RAT. No good Rust implementation; no good open-source Lazard-style implementation outside Maple.

**DEFER to v2, 8–12 PM** — large, doubly-exponential, important only for real quantifier elimination / nonlinear arithmetic users.

### 4.5 Real root isolation
Vincent–Akritas–Strzeboński continued fractions (VAS-CF) is the *Mathematica* default and is generally the fastest in practice; the Descartes/bisection method (with Mahler–Davenport bounds) is more uniformly bounded. msolve and FLINT (`arb_fmpz_poly_complex_roots`, `fmpz_poly_roots`) both ship state-of-the-art certified isolators. A Rust crate `find-real-roots-of-polynomial` exists (Sturm-chain over `BigRational`) but is single-author, MIT, and not performance-competitive. **GO via FFI to FLINT (1 PM)** plus a small native fallback.

---

## 5. Simplification & Normal Forms

### 5.1 D-finite / holonomic function manipulation
Chyzak–Salvy `Mgfun`/`gfun` Maple packages (1990s), Koutschan `HolonomicFunctions` Mathematica package; the framework supports closure properties (sum/product/substitution of holonomic functions), creative telescoping, recurrence solving. This subsumes a huge fraction of special-function manipulation (Bessel, hypergeometric, orthogonal polynomials). **GO native, ~3–5 PM** as part of the summation/definite-integration engine.

### 5.2 Algebraic numbers (RootOf), LLL-based recognition
Trager's algorithm for arithmetic in algebraic extensions (1976, 1984); LLL-based recognition of constants from numerical approximations (PSLQ, `IntegerRelations`). FLINT's `ca` (Calcium) module is the reference: it represents elements of the field generated by symbolic constants and does decidable equality testing within that closure. Per the Calcium docs, it supports arithmetic in `Q-bar`, transcendental extensions by `exp`/`log`, etc. Uray (arXiv:1810.01634; *J. Symb. Comp.* 2023) gives a polynomial bound on LLL with algebraic-number coordinates.

**GO via FFI to FLINT `ca` + `fq` (1 PM).** `lll-rs` exists as a Rust crate but the authors themselves note: "lll-rs is far from feature-complete and should be considered experimental. Users willing to use a stable and battle-tested library should consider fplll instead." `fplll` (LGPL) is the right backend if FLINT's LLL isn't sufficient.

### 5.3 Polynomial GCD
The relevant landscape:
- Dense modular (Brown), bivariate sparse (Zippel) — 1970s–80s classics.
- Ben-Or/Tiwari sparse interpolation (1988), key building block.
- Hu–Monagan, "A Fast Parallel Sparse Polynomial GCD Algorithm" (ISSAC '16; full version *J. Symb. Comp.* 105, 2021, 28–63) — Kronecker substitution + modified Ben-Or/Tiwari modulo a smooth prime + parallelization in Cilk. From Hu's PhD: on a benchmark with `#G ≈ 10^4, #A ≈ #B ≈ 10^6`, Maple takes 22,111 s, Magma 1,611 s, the new algorithm 4.67 s.
- Huang–Monagan, "A New Sparse Polynomial GCD by Separating Terms" (ISSAC '24, doi:10.1145/3666000.3669684) — further factor-of-many speedup by interpolating the smaller of `gcd(A,B)` and the cofactor `A/G`.

FLINT (and msolve's parallel routines) implement competitive variants. **GO via FFI to FLINT for v1 (0.5 PM); GO native sparse later (~4 PM)** if benchmarks demand it.

### 5.4 Polynomial factorization
- Univariate over `Z`: Berlekamp–Zassenhaus + van Hoeij's LLL-based knapsack recombination (van Hoeij, *J. Number Theory* 95, 2002, 167–189; Hart–van Hoeij–Novocin, "Practical Polynomial Factoring in Polynomial Time," 2010). Per Klüners' survey: "About 40 years ago Hans Zassenhaus developed an algorithm … This algorithm worked very well for many examples, but his worst case complexity was exponential. … The resulting knapsack problem can be efficiently solved using lattices and the LLL algorithm." FLINT (`fmpz_poly_factor`) implements this and is the current world record on adversarial inputs.
- Univariate over `Fq`: Cantor–Zassenhaus, Shoup–Kaltofen subquadratic.
- Multivariate: Hensel-lifting + recombination (LeBrun, Lecerf); sparse factorization via Monagan's algorithms.

**GO via FFI to FLINT (0.5 PM).** Re-implementing factorization in Rust is not justified by ROI; FLINT is decades ahead.

### 5.5 Algebraic simplification with side relations
Standard Gröbner-basis normal form once you have a Gröbner engine. **GO native, 1 PM** on top of the Gröbner module.

---

## 6. Future work (mentioned only)
**ODE/PDE solvers — NO-GO for v1.**
- Kovacic's algorithm (Liouvillian solutions of 2nd-order linear ODE, 1986); rich extensions by van Hoeij, Singer, van der Put.
- Prelle–Singer for first-order ODE (1983); active research line (e.g., Avellar et al.).
- Lie symmetry methods (Olver, Hereman; Maple `PDEtools/Lie`).

These are major projects in their own right, badly served by all open-source CAS (SymPy's `dsolve` is the most complete, and is still incomplete). Their place in the roadmap is after the integration + summation tower is solid.

---

## Cross-cutting Infrastructure Decisions

### Arbitrary precision integers and rationals
**Use `rug` for v1** (LGPL via `gmp-mpfr-sys`), and structure the codebase to abstract over a `BigInt` trait so you can swap to `malachite` later. Concrete benchmark data from `bigint-benchmark-rs` (Tomek Czajka):
- `num-bigint`: pure Rust, MIT/Apache, slowest.
- `malachite`: pure Rust, **LGPL 3.0**, performance "faster than num due to better algorithms, and slower than rug" (per malachite.rs/performance). FFT multiplication still trails GMP.
- `rug` / `gmp-mpfr-sys`: FFI to GMP, **LGPL 3.0**, fastest, mature.

Licensing irony: `malachite` is itself LGPL because it ports GMP/MPFR algorithms (per its README — "Parts of Malachite are derived from GMP, FLINT, and MPFR"). So you do not escape LGPL by going pure-Rust unless you start from `num-bigint` and accept the perf hit.

**For finite fields**, do *not* use `ark-ff` for runtime-chosen moduli — it requires the modulus at compile time via `#[modulus = "..."]`. Use FLINT's `nmod`/`fq` or write a small Montgomery-arithmetic module in Rust for word-size primes (this is where SIMD via `wide` or `std::simd` and `rayon`-based parallel Gaussian elimination over `Z/pZ` gives Rust a real advantage — see msolve's parallel linear algebra design).

### Polynomial representation
Three representations needed:
1. **Univariate dense `Vec<Coef>`** for `fmpz_poly`-style work; trivial.
2. **Sparse distributed** (sorted `Vec<(Exp, Coef)>` with monomial as packed `u64` or `[u64; N]`) for multivariate. This is the representation Symbolica, msolve, FLINT (`fmpz_mpoly`) all use; the monomial hashtable is critical for F4. Rust's borrow checker is *helpful* here — bump-allocated arenas plus zero-copy iteration over coefficient slices is idiomatic and safe.
3. **Recursive `Poly[x_n][x_{n-1}]...[x_1]`** for Risch differential-tower work, where the *order of variables matters* (it is the monomial tower). Use an enum/box recursion; performance is fine because the per-node work is large.

### Expression DAG and hash-consing
Maxima's biggest structural weakness is its untyped Lisp list representation with no canonical hash-consing. **Use a hash-consed interned expression DAG** with `Arc<AtomNode>` or an arena-based `u32` index (Symbolica uses the latter for cache locality). The borrow checker is *helpful* for invariants ("no node mutated after interning"); the lack of GC is *neutral* because expressions are acyclic, so reference counting suffices.

### Numerical/symbolic interop
- **Ball arithmetic via FLINT (`arb`/`acb`)**: the gold standard for rigorous numerics, exposed through `flint3-sys`. From the FLINT docs: "FLINT has advanced support for real and complex numbers, implemented using ball arithmetic. It covers a variety of numerical functionality … with arbitrary precision and with rigorous error bounds." Use `arb` for: zero-testing in Gruntz / Risch, certified definite integration, certified root isolation in CAD.
- **BLAS/LAPACK** via `nalgebra` + `lapack-sys`, or `ndarray` + `ndarray-linalg`, for dense matrix work in FGLM and CAD lifting.
- **Interval arithmetic** native Rust: `inari` (IEEE-1788), `gauss-rs` — adequate for fast heuristic bounds; switch to Arb when rigor is required.

---

## What FriCAS / AXIOM Did Right That Maxima Missed

The FriCAS/AXIOM advantage is not algorithmic — it is *type-theoretic*. SPAD (FriCAS) and Aldor (its successor) implement a two-level hierarchy:

> "Domains are comparable to classes in object oriented programming languages. Categories are somewhat comparable to interfaces in Java, but are much more powerful." (FriCAS wiki, *ProgrammingSPAD*)

> "A category only describes the signatures of functions that domains that belong to this category must provide. Categories are organised in hierarchies. A category can inherit from several other categories. Multiple inheritance is not a problem since categories only describe the interface of domains but do not implement any function themselves." (FriCAS-Types notebook)

Per the FriCAS Wikipedia article (en.wikipedia.org/wiki/FriCAS), FriCAS ships "a rich library comprising over 1,260 constructors and 8,326 operations" across this hierarchy (Ring → CommutativeRing → IntegralDomain → GcdDomain → EuclideanDomain → Field; OrderedSet → DifferentialRing → FunctionSpace → ExpressionSpace; etc.). The pay-off is *generic algorithms*: the file `src/algebra/ore.spad` shows conditional exports — "`if R has IntegralDomain then … if R has GcdDomain then content l == gcd coefficients l; … if R has Field then …`" — i.e., the same Ore-polynomial constructor specializes automatically as its coefficient ring acquires more structure.

The integration code makes this concrete. From the FriCAS source:
- `ElementaryIntegration(R, F)` requires `R : Join(GcdDomain, Comparable, CharacteristicZero, PolynomialFactorizationExplicit, …)`
- `FunctionSpaceIntegration(R, F)` requires the same.
- `RationalFunctionIntegration(F)` requires `F : Join(IntegralDomain, RetractableTo Integer, CharacteristicZero)`.

This is what makes "the most complete Risch implementation" practical: Bronstein's algorithm is genuinely polymorphic over the differential ring of constants and the coefficient field; FriCAS expresses that polymorphism directly, Maxima cannot.

**Concrete Maxima limitations FriCAS handles** (from the FriCAS `FriCASIntegration` wiki, maxima-discuss mailing list, and the Stanford SURIM 2025 report by Gao & Yeo):
1. `integrate(sqrt(x^2+1)/(x^3+1), x)` — Maxima 5.45 fails; FriCAS returns an elementary antiderivative.
2. `integrate(x/sqrt(x^4 + 10*x^2 - 96*x - 71), x)` — Maxima fails; AXIOM/FriCAS solves it.
3. `integrate((x^2+2*x-2)/(x*(x-2)*y), x)` with `y^2 = x^3 + 1` — a pure algebraic case; only Risch–Trager–Bronstein systems handle it.
4. Generally: integration where the coefficient field is itself an algebraic extension, where the differential tower has multiple nested exponentials/logarithms, and where polynomial factorization over algebraic-number coefficients is needed mid-Risch.

A maxima-discuss mailing-list characterization captures the design gap bluntly: *"Maxima is what is called a CAS of the 2nd generation. (As are Mma, Maple and most other popular CAS). Axiom is a CAS of the 3rd generation."* The third-generation feature is the category system.

**For the Rust refactor:** Rust *traits* are exactly the right tool to express SPAD categories, and the trait coherence rules (no orphan impls, blanket impls) map well to category inheritance. Symbolica's lead developer Ben Ruijl remarked on the design experience in "Symbolica 1.0: Symbolic mathematics in Rust + two new open-source crates" (symbolica.io/posts/stable_release/): "Rust has been tremendously helpful to get this project off the ground. Not having to worry about memory corruption, the ease of zero-cost abstractions and pyo3 for Python binding generation have sped up development. … It is not possible yet to have mutually exclusive traits, leading to some limitations related to overlapping blanket implementations." The lack of trait specialization in stable Rust is the one place SPAD beats Rust today — but it is a tractable engineering problem (associated-type-bounds workarounds, dispatch via marker traits, or `min_specialization` on nightly).

**Strategic recommendation:** design the algebraic-structure trait hierarchy first, before the integrator. Treat it as your equivalent of the FriCAS category library — `Ring`, `CommutativeRing`, `IntegralDomain`, `GcdDomain`, `EuclideanDomain`, `Field`, `DifferentialRing`, `FunctionSpace`, `ExpressionSpace`, `OrePolynomialRing`. Get this right and the integration/summation algorithms will be straightforward generic code on top.

---

## Rust Ecosystem and Licensing Summary

### Crates surveyed
| Crate | Scope | License | Notes |
|---|---|---|---|
| `symbolica` (Ben Ruijl) | full CAS, fast multivariate poly | **source-available, commercial** | unusable as dependency; cannot redistribute |
| `numerica`, `graphica` | extracted from Symbolica v1.0 | MIT | usable; covers some numeric kernels |
| `mathru` | numerical linear algebra | MIT/Apache | NOT a CAS — numerical only |
| `cas-rs` | calculator/REPL | permissive | very early; "early stage of development" |
| `rusymbols`, `mathcore`, `symbolic_math` | toy symbolic | permissive | not production |
| `flint3-sys`, `flint-sys` | FLINT FFI | MIT (binding); FLINT = LGPL 3+ | the workhorse |
| `rug`, `gmp-mpfr-sys` | GMP/MPFR/MPC | LGPL 3 | de facto big-num |
| `malachite` | pure-Rust big-num | **LGPL 3** | derived from GMP/FLINT/MPFR source |
| `num-bigint` | pure-Rust big-num | MIT/Apache | only permissive option, slowest |
| `ark-ff` | finite fields | MIT/Apache | compile-time moduli; bad fit for CAS |
| `nalgebra`, `ndarray`, `faer` | dense linear algebra | permissive | fine for FGLM |
| `lll-rs` | LLL | permissive | "experimental … consider fplll instead" |
| `inari` | IEEE-1788 intervals | permissive | adequate for heuristic bounds |
| `find-real-roots-of-polynomial` | Sturm isolation | MIT | toy |

### Licensing of upstream C/C++ libraries
| Library | License | FFI impact for permissive Rust port |
|---|---|---|
| FLINT (≥3.1) | **LGPL v3+** | OK with dynamic linking / relinkable objects |
| Arb (now folded into FLINT 3) | LGPL 2.1+ → LGPL 3+ | OK as above |
| GMP, MPFR, MPC | LGPL 3+ (GMP dual LGPL 3 / GPL 2) | OK as above |
| NTL | LGPL 2.1+ | OK |
| **msolve** | **GPLv2+** | **infects any linked binary** |
| **Singular** | **GPL v2 or v3** | **infects** |
| **PARI/GP** | **GPL v2+** | **infects** |
| Maxima itself | GPLv2+ | Your refactor's license is **unspecified** in the prompt — if you want MIT/Apache you must clean-room reimplement, not translate Maxima Lisp |

**Decision implication:** if the refactor must remain GPL-compatible (the default if you genuinely "refactor" Maxima rather than rewrite), wrap msolve and PARI freely. If the refactor wants a permissive license, the polynomial-system path goes through a native F4 in Rust (Groebner.jl-class effort) plus FLINT FFI for everything else, plus a clean-room rewrite that avoids copying Maxima Lisp idioms.

---

## Numerical Backend Integration

For a CAS that needs to bridge symbolic and numeric:

1. **Ball/interval arithmetic — use Arb via FLINT.** Already covered by `flint3-sys`; the ball arithmetic API is exactly what you need for certified zero-testing in Gruntz, Risch constant problems, and CAD sample points. FLINT 3 unified `arb`, `acb`, `ca` into a single library, so one FFI dependency covers all.
2. **BLAS/LAPACK** — wire `nalgebra` to `lapack-sys` (or `intel-mkl-src` behind a feature flag) for the dense matrix work in FGLM and CAD lifting; use `rayon` for embarrassingly parallel work (multi-modular GB, parallel Hensel lifting).
3. **Rust-native alternatives:** `nalgebra` for dense up to ~1000×1000 over `f64`; `faer` (newer, pure-Rust, claims competitive with MKL) is worth tracking. For exact linear algebra over `Z/pZ`, write your own — this is where Rust + SIMD shines and where msolve/FLINT have spent their optimization budget.
4. **GPU/codegen:** Symbolica's "Fast code generation (C++/ASM/SIMD/CUDA) for expression evaluation" is a model worth emulating for the numeric-evaluation path. The kernel/evaluator you have already built is the right substrate.

---

## Recommendations (staged)

**Phase 0 — Trait/type hierarchy (3 PM).** Build the SPAD-style trait tower: `Ring → IntegralDomain → GcdDomain → EuclideanDomain → Field`, `DifferentialRing`, `FunctionSpace`, `ExpressionSpace`, `OrePolynomial<R>`. Hash-consed expression DAG with arena allocation. Decide the licensing posture (MIT/Apache vs GPL); this gates everything downstream.

**Phase 1 — Polynomial / number core via FFI (3 PM).** Wrap FLINT (`fmpz_poly`, `fmpq_poly`, `fmpz_mpoly`, `nmod_poly`, `fq`, `ca`, `arb`, `acb`) behind your own traits. Wrap msolve only if GPL is acceptable. Wrap `fplll` for LLL.

**Phase 2 — Rational + Risch transcendental (10–14 PM).** Implement Hermite, LRT, Bronstein's transcendental integrator from the 2005 book, plus Risch–Norman heuristic as a fast path. Benchmark against FriCAS on Sutherland's algebraic suite and the RUBI test set.

**Phase 3 — Limits + summation (10 PM).** Gruntz limits with a robust series engine; Gosper, Zeilberger, Karr, Almkvist–Zeilberger, Koutschan-style holonomic closures.

**Phase 4 — Risch algebraic + simplification depth (8 PM).** Trager's radical-only subset, then Bronstein's algebraic extensions.

**Phase 5 — Native F4 (if permissive license needed) or wider Maple-feature parity (12 PM).**

**Phase 6+ — Differential equations, CAD, transseries, Meijer-G, special-function integration.** Each is its own multi-PM project; sequence by user demand.

**Benchmarks/triggers that would change these recommendations:**
- If FLINT's `ca` (Calcium) becomes the de facto algebraic-closure representation (it is moving that way as of FLINT 3.x), wrap it instead of rolling your own RootOf.
- If `Groebner.jl` publishes a stable C API or a Rust binding appears, prefer it over native F4.
- If Symbolica relicenses (unlikely), reconsider using it as a polynomial backend.
- If SciML's `SymbolicIntegration.jl` reaches feature parity with FriCAS, study (don't link) its Bronstein-book port — it is the cleanest open code in the world for this material.

---

## Caveats

- The "most complete Risch implementation" claim for FriCAS is the FriCAS project's own characterization (wiki, features page) and is echoed by Wikipedia and academic surveys, but there is no peer-reviewed benchmark paper that *quantitatively* ranks integrators across the full Bronstein-book scope. The arXiv:2004.04910 pseudo-elliptic benchmark is a useful but narrow data point.
- Schneider's `Sigma`, Koutschan's `HolonomicFunctions`, and the entire RISC particle-physics pipeline are closed-source / password-protected. Reading the published algorithms is fine; cloning the *code* is not.
- Effort estimates assume a single experienced systems engineer with prior CAS exposure, working full-time, and willing to read Bronstein, Geddes/Czapor/Labahn, *A=B*, and the relevant ISSAC papers. Add 50% for context-switching and team scaling.
- Risch is a semi-algorithm: every implementation in existence relies on heuristics or oracle calls for constant-field zero-testing. Treat any "completeness" claim as conditional on those oracles.
- The Rust ecosystem for CAS is genuinely thin in May 2026 — Symbolica is the only production-quality entry, and it is not redistributable. This is both the risk and the opportunity for the refactor.
- The crate `calcu-rs` was mentioned in the prompt but could not be located on crates.io, lib.rs, or GitHub topic search; flagging the name as possibly stale or referring to a private/unpublished project — `cas-rs` (ElectrifyPro) is the closest extant match and is itself "in a very early stage of development."