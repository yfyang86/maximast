# RC3–RC6 Completion Status (Final)

## Phase 1: Core Strengthening — **DONE**

- [x] ratsimp via polynomial GCD cancellation
- [x] partfrac (distinct linear factors)
- [x] Transitive inference (a<b, b<c ⟹ a<c)
- [x] declare + property tracking (featurep)

## Phase 2: Solving + Matrices — **DONE**

- [x] linsolve (Gaussian elimination)
- [x] eigenvalues via charpoly + factor
- [x] eigenvectors (n×n via null space)
- [x] rank (numeric row echelon)
- [x] solve cubic/quartic/higher (via factoring)

## Phase 3: Integration Depth — **DONE**

- [x] Integration by parts (x*exp, x*sin, x*cos, x*log, x^n*exp)
- [x] Rational function integration (partfrac → log terms, 1/(ax+b)^n)
- [x] L'Hôpital for limits (0/0 form)
- [x] Limits at infinity (polynomial/rational degree)
- [x] Gruntz foundation (exp/log growth)
- [x] 1^∞ indeterminate form → exp(g*log(f))
- [x] sec/csc integration
- [x] log^n integration (n=2,3)
- [x] x/f(x) pattern (derivative recognition → log)
- [x] Completing the square → atan for 1/(ax²+bx+c)

## Phase 4: I/O + Display — **DONE**

- [x] File loading (load/batch/batchload)
- [x] TeX output (tex function)
- [x] Grind (1D output)
- [x] REPL with readline (arrow keys, history, syntax highlighting)
- [x] README.md with full usage guide

## Gaps vs Full Maxima

- [ ] 2D ASCII display (fraction bars, exponent layout)
- [ ] Full Risch algorithm (transcendental integration)
- [ ] Full Gruntz (MRV for nested exponentials)
- [ ] Berlekamp/Hensel factoring
- [ ] Trig identities (sin²+cos²=1)
- [ ] Special functions, numerical methods, plot
