# v5.0 Sprint Index

## Status: ✅ All 18 Sprints Complete

## Goal

Standard library expansion — fill the most commonly needed Maxima functions
that real scripts depend on. Ordered by impact × cost.

## Sprints

| Sprint | Content | Size | Status |
|--------|---------|------|--------|
| **S1** | Sets: union, intersection, setdifference, powerset | Small | ✅ Done |
| **S2** | Strings: slength, substring, ssearch, split | Small | ✅ Done |
| **S3** | Number theory: ifactors, totient, divisors, next_prime, CRT, fibonacci | Small | ✅ Done |
| **S4** | Polynomial: resultant, discriminant, content, primpart | Medium | ✅ Done |
| **S5** | Log: logcontract, logexpand | Medium | ✅ Done |
| **S6** | Expression: multthru, xthru, collectterms, at, lopow | Small | ✅ Done |
| **S7** | Laplace transforms: laplace, ilt (table-driven) | Large | ✅ Done |
| **S8** | ODE solver: ode2 (separable, linear, const-coeff) | Large | ✅ Done |
| **S9** | Complex numbers: %i^2→-1, realpart, imagpart, conjugate | Small | ✅ Done |
| **S10** | Trig special values: sin(%pi/6)→1/2, full table | Small | ✅ Done |
| **S11** | Matrix element-wise: +, -, scalar* | Small | ✅ Done |
| **S12** | Display: 1/x not x^(-1), fraction form | Small | ✅ Done |
| **S13** | Partfrac: irreducible quadratics, biquadratic factoring | Medium | ✅ Done |
| **S14** | Non-homogeneous ODE: undetermined coefficients | Medium | ✅ Done |
| **S15** | Sturm chains: nroots, realroots | Medium | ✅ Done |
| **S16** | Pattern matching: matchdeclare, defrule, apply1, tellsimp | Large | ✅ Done |
| **S17** | bfloat: floating-point evaluation | Large | ✅ Done |
| **S18** | Plotting: plot2d (SVG), gnuplot_script | Large | ✅ Done |

## Phases

| Phase | Sprints | Est. Time | Focus |
|-------|---------|-----------|-------|
| **Phase 1** | S1, S2, S3, S6 | ~8 hours | Quick wins — fill obvious gaps |
| **Phase 2** | S4, S5 | ~6 hours | Core math infrastructure |
| **Phase 3** | S7, S8 | ~13 hours | Advanced: transforms and ODEs |

## Key Metrics (target)

| Metric | Before | After |
|--------|--------|-------|
| Tests | 822 | ~950+ |
| Functions | ~189 | ~250+ |
| Walkthroughs | 16 | ~20 |

## Documents

| File | Contents |
|------|----------|
| [PLAN.md](PLAN.md) | Full sprint plan with task checklists and verification |
