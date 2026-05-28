# Maxima Kernel v2.0 — Overview

## Goal

Implement proper algorithmic foundations for the Maxima Rust kernel,
replacing v1.0 heuristic workarounds with real CAS algorithms.

## Status: Complete (2026-05-25)

All 8 core sprints (S1-S8) substantially complete. 72/87 tasks done (83%).

## v1.0 → v2.0 Comparison

| Area | v1.0 Status | v2.0 Status |
|------|-------------|-------------|
| Simplifier | 751 lines, basic rules | + Pythagorean, De Morgan, trig powers |
| Integration | ~50 table patterns | + Hermite, partfrac, Risch tower, substitution, 55+ patterns |
| Limits | L'Hôpital + growth heuristic | + MRV Gruntz, series, 0*∞, iterated L'Hôpital |
| Def. Integration | FTC only | + Infinite bounds, Gaussian, residues |
| Summation | Numeric iteration only | + Faulhaber, Gosper, geometric, telescoping |
| Poly/GCD | Basic Euclidean | + Rational coeff normalization, CRE type |

## v2.0 Sprints

| Sprint | Content | Status |
|--------|---------|--------|
| **S1** | Trig simplification | ✅ Complete |
| **S2** | Trait hierarchy + CRE | ✅ Complete |
| **S3** | Risch-Norman heuristic | ✅ Complete |
| **S4** | Complete rational integration | ✅ Complete |
| **S5** | Risch transcendental | ✅ Complete |
| **S6** | Gruntz + series engine | ✅ Complete |
| **S7** | Gosper summation | ✅ Complete |
| **S8** | Definite integration | ✅ Complete |
| **S9** | Risch algebraic | Deferred |

## Test Coverage

- **644 total tests** (404 eval + 106 poly + 78 parser + 43 core + 13 integration)
- **Integration formula suite**: 51/59 non-parametric formulas pass (86.4%)
- **rtest1**: 164/208 (79%)

## Branch

- v1.0: branch `v1.0` (frozen at commit 4ae28ba)
- v2.0: development on `dev` (PR #82)

## New Source Modules

| File | Lines | Purpose |
|------|-------|---------|
| `crates/eval/src/series.rs` | 257 | Truncated power series |
| `crates/eval/src/risch_tower.rs` | 193 | Differential field tower |
| `crates/eval/src/risch_integrate.rs` | 355 | Risch integration |
| `crates/eval/src/gruntz.rs` | 600 | MRV Gruntz algorithm |
| `crates/eval/tests/formula_test.rs` | 169 | Formula test suite |
| `crates/poly/src/cre.rs` | 224 | Canonical Rational Expressions |
| `crates/poly/src/traits.rs` | 150 | Algebraic trait hierarchy |
