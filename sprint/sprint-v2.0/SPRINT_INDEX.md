# v2.0 Sprint Index

## Goal

Implement proper algorithmic foundations for the Maxima Rust kernel:
Risch integration, Gruntz limits, Gosper summation, definite integration.

## Sprint Status (2026-05-25)

| Sprint | Content | Status |
|--------|---------|--------|
| **S1** | Trig simplification (Pythagorean, De Morgan) | ✅ Complete |
| **S2** | Trait hierarchy + CRE (Ring→Field, CRE arithmetic) | ✅ Complete |
| **S3** | Risch-Norman heuristic (ansatz, coeff matching) | ✅ Complete |
| **S4** | Complete rational integration (Hermite, mixed partfrac) | ✅ Complete |
| **S5** | Risch transcendental (tower, primitive/exp case, RDE) | ✅ Complete |
| **S6** | Gruntz + series (MRV, series type, exp rewrite) | ✅ Complete |
| **S7** | Gosper summation (Faulhaber, geometric, telescoping) | ✅ Complete |
| **S8** | Definite integration (infinite bounds, residues, Gaussian) | ✅ Complete |
| **S9** | Advanced formulas + Risch algebraic | Deferred |

**All 8 core sprints complete. 72/87 tasks done (83%).**

## Documents

| File | Contents |
|------|----------|
| [REVISED_PLAN.md](REVISED_PLAN.md) | Full sprint plan with task checklists and status |
| [OVERVIEW.md](OVERVIEW.md) | Goals, scope, baseline |
| [S1_TRIG_SIMPLIFICATION.md](S1_TRIG_SIMPLIFICATION.md) | Trig identities |
| [S2_RISCH_RATIONAL.md](S2_RISCH_RATIONAL.md) | CRE, Hermite reduction |
| [S3_FULL_LIMITS.md](S3_FULL_LIMITS.md) | Gruntz algorithm |
| [S4_DEFINITE_INTEGRATION.md](S4_DEFINITE_INTEGRATION.md) | Residues, definite integrals |
| [RISCH_GRUNTZ_IMPLEMENTATION.md](RISCH_GRUNTZ_IMPLEMENTATION.md) | Deep infrastructure notes |

## Test Results

| Metric | Count |
|--------|-------|
| Total workspace tests | 644 |
| eval unit tests | 404 |
| Integration formula pass rate | 86.4% (51/59) |
| rtest1 pass rate | 79% (164/208) |

## Key Capabilities Added in v2.0

### Integration
- Hermite reduction for repeated linear factors
- Mixed linear + irreducible quadratic partial fractions
- Integration by substitution (log/exp compositions, power substitution)
- Risch tower construction + primitive/exponential case
- 25+ trig/hyperbolic/inverse-trig table formulas
- sin⁴, cos⁴ power reduction
- x^n*log(x), x*exp(ax) by-parts formulas

### Limits
- Proper MRV Gruntz algorithm with series expansion
- Exponential dominance (exp(x)/x^n → ∞)
- 0*∞ handler (x*sin(1/x) → 1)
- Iterated L'Hôpital (up to 5 iterations)
- log(log(x))/log(x) → 0

### Summation
- Polynomial sums (Faulhaber: Σk, Σk², Σk³)
- Geometric series
- Telescoping detection + partial fraction decomposition
- Arith-geometric (Σk*r^k)
- Gosper's algorithm with certificate verification

### Definite Integration
- Infinite bounds via Gruntz limits
- Known integrals (Gaussian, exponential)
- Residue method for products of quadratics
- atan(±∞) = ±π/2
