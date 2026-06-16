//! Recursive multivariate GCD over Q (primitive PRS), the complete replacement
//! for the incomplete Kronecker GCD (V12 P2).
//!
//! gcd(a,b) in Q[x_1,…,x_n]: pick a main variable v, split content (a recursive
//! gcd of the coefficients in the remaining variables) from primitive part, run
//! a primitive polynomial-remainder-sequence Euclidean algorithm in v, and
//! recombine. Coefficients are over the field Q, so this is exact and complete:
//! `gcd(x^2-y^2, (x+y)^2) = x+y` (which Kronecker returned as a noun).

use crate::mpoly::{MPoly, Monomial, MCoeff, make_monic};
use num::One;

fn degree_in(p: &MPoly, v: usize) -> u32 {
    p.terms.iter().map(|(m, _)| m.0[v]).max().unwrap_or(0)
}

/// Coefficient of v^e: the terms with exponent e in variable v, with that
/// exponent zeroed (an MPoly that does not involve v).
fn coeff_of_v(p: &MPoly, v: usize, e: u32) -> MPoly {
    let mut terms = Vec::new();
    for (m, c) in &p.terms {
        if m.0[v] == e {
            let mut mm = m.0.clone();
            mm[v] = 0;
            terms.push((Monomial(mm), c.clone()));
        }
    }
    let mut r = MPoly { vars: p.vars.clone(), order: p.order, terms };
    r.canonicalize();
    r
}

/// The monomial v^k (over `nvars` variables).
fn mono_v(nvars: usize, v: usize, k: u32) -> Monomial {
    let mut e = vec![0u32; nvars];
    e[v] = k;
    Monomial(e)
}

/// content w.r.t. v: gcd (recursive, in the remaining variables) of all the
/// coefficients of v^e.
fn content_v(a: &MPoly, v: usize) -> MPoly {
    let mut g: Option<MPoly> = None;
    for e in 0..=degree_in(a, v) {
        let c = coeff_of_v(a, v, e);
        if c.is_zero() { continue; }
        g = Some(match g {
            None => c,
            Some(g) => gcd_rec(&g, &c),
        });
    }
    g.unwrap_or_else(|| MPoly::constant(a.vars.clone(), a.order, MCoeff::one()))
}

fn primpart_v(a: &MPoly, v: usize) -> MPoly {
    let c = content_v(a, v);
    a.exact_div(&c).unwrap_or_else(|| a.clone())
}

/// Pseudo-remainder of a by b, treated as polynomials in v (deg_v(a) ≥ deg_v(b)).
fn pseudo_rem(a: &MPoly, b: &MPoly, v: usize) -> MPoly {
    let nvars = a.nvars();
    let db = degree_in(b, v);
    let lcb = coeff_of_v(b, v, db);
    let mut r = a.clone();
    let mut e: i64 = degree_in(a, v) as i64 - db as i64 + 1;
    if e < 0 { e = 0; }
    while !r.is_zero() && degree_in(&r, v) >= db {
        let dr = degree_in(&r, v);
        let lcr = coeff_of_v(&r, v, dr);
        // term = lcr · v^(dr−db) · b
        let term = b.monomial_mul(&MCoeff::one(), &mono_v(nvars, v, dr - db)).mul(&lcr);
        // r = lcb·r − term  (cancels the v^dr leading term)
        r = r.mul(&lcb).sub(&term);
        e -= 1;
    }
    for _ in 0..e { r = r.mul(&lcb); }
    r
}

/// gcd of primitive parts via a primitive PRS Euclidean loop in v.
fn prs_gcd(mut a: MPoly, mut b: MPoly, v: usize) -> MPoly {
    if degree_in(&a, v) < degree_in(&b, v) {
        std::mem::swap(&mut a, &mut b);
    }
    loop {
        if b.is_zero() { return a; }
        let r = pseudo_rem(&a, &b, v);
        a = b;
        b = if r.is_zero() { r } else { primpart_v(&r, v) };
    }
}

/// Recursive multivariate GCD (monic). Result divides both inputs and is their
/// greatest common divisor.
pub fn gcd_rec(a: &MPoly, b: &MPoly) -> MPoly {
    if a.is_zero() { return make_monic(b); }
    if b.is_zero() { return make_monic(a); }
    let nvars = a.nvars();
    // Main variable: any with positive degree in either operand.
    let v = (0..nvars).find(|&v| degree_in(a, v) > 0 || degree_in(b, v) > 0);
    let Some(v) = v else {
        // Both are nonzero constants → unit gcd.
        return MPoly::constant(a.vars.clone(), a.order, MCoeff::one());
    };
    let ca = content_v(a, v);
    let cb = content_v(b, v);
    let pa = a.exact_div(&ca).unwrap_or_else(|| a.clone());
    let pb = b.exact_div(&cb).unwrap_or_else(|| b.clone());
    let cont = gcd_rec(&ca, &cb);
    let g = primpart_v(&prs_gcd(pa, pb, v), v);
    make_monic(&cont.mul(&g))
}

#[cfg(test)]
mod tests {
    use super::gcd_rec;
    use crate::{expr_to_mpoly, MPoly, MonomialOrder};
    use maxima_core::{Expr, intern, SymbolId};

    fn vars() -> Vec<SymbolId> { vec![intern("x"), intern("y")] }
    fn mp(e: &Expr) -> MPoly { expr_to_mpoly(e, &vars(), MonomialOrder::Grevlex).unwrap() }
    fn x() -> Expr { Expr::sym("x") }
    fn y() -> Expr { Expr::sym("y") }
    fn sq(e: Expr) -> Expr { Expr::pow(e, Expr::int(2)) }

    #[test] fn gcd_perfect_square_case() {
        // gcd(x²−y², (x+y)²) = x+y  — the case Kronecker GCD returned as a noun.
        let a = mp(&Expr::sub(sq(x()), sq(y())));
        let b = mp(&sq(Expr::add(x(), y())));
        assert_eq!(gcd_rec(&a, &b), mp(&Expr::add(x(), y())));
    }
    #[test] fn gcd_difference_of_cubes() {
        // gcd(x³−y³, x²−y²) = x−y
        let a = mp(&Expr::sub(Expr::pow(x(), Expr::int(3)), Expr::pow(y(), Expr::int(3))));
        let b = mp(&Expr::sub(sq(x()), sq(y())));
        assert_eq!(gcd_rec(&a, &b), mp(&Expr::sub(x(), y())));
    }
    #[test] fn gcd_coprime_is_unit() {
        // gcd(x+y, x−y) = 1 (genuinely coprime — now detected, not a noun).
        let g = gcd_rec(&mp(&Expr::add(x(), y())), &mp(&Expr::sub(x(), y())));
        assert!(g.lm().map_or(true, |m| m.is_one())); // constant
    }
}
