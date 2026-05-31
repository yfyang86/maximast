//! Residues of rational/meromorphic functions at a pole.
//!
//!   residue(f, z, z0):
//!     simple pole  (order 1): N(z0)/D'(z0)
//!     order-m pole (m >= 2):  (1/(m-1)!) * d^{m-1}/dz^{m-1}[ (z-z0)^m f ] |_{z=z0}
//!
//! The pole order is found by differentiating the denominator and testing for
//! zeros at z0 (works for complex z0 such as %i because the simplifier folds
//! %i^2 -> -1). (z-z0)^m is cancelled out of the denominator by repeated
//! synthetic division rather than complex factorization.

use maxima_core::Expr;
use crate::simp::simplify;
use crate::eval::{meval, expand, diff_once, extract_fraction};
use crate::helpers::{contains_var, subst};
use crate::complex::complex_div;
use crate::poly_expr::PolyExpr;

const MAX_POLE_ORDER: usize = 12;

pub(crate) fn eval_residue(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    if name != "residue" || args.len() != 3 {
        return None;
    }
    let f = &args[0];
    let z = &args[1];
    let z0 = &args[2];
    if !matches!(z, Expr::Symbol(_)) {
        return None;
    }
    Some(residue(f, z, z0, env))
}

fn residue(f: &Expr, z: &Expr, z0: &Expr, env: &mut crate::env::Environment) -> Expr {
    let noun = || Expr::call("residue", vec![f.clone(), z.clone(), z0.clone()]);

    // Split into numerator / denominator.
    let (num, den) = match extract_fraction(f) {
        Some((n, d)) => (n, d),
        None => (f.clone(), Expr::int(1)),
    };

    // No dependence on z in the denominator → analytic → residue 0.
    if !contains_var(&den, z) {
        return Expr::int(0);
    }

    // Determine the pole order m at z0 by the derivative test on the denominator.
    let m = match pole_order(&den, z, z0, env) {
        Some(m) => m,
        None => return noun(),
    };
    if m == 0 {
        return Expr::int(0); // not a pole
    }

    if m == 1 {
        // Simple pole: residue = N(z0) / D'(z0).
        let n_at = eval_at(&num, z, z0, env);
        let dprime = diff_once(&den, z);
        let d_at = eval_at(&dprime, z, z0, env);
        if d_at == Expr::int(0) {
            return noun(); // shouldn't happen for m==1, but stay safe
        }
        return complex_div(&n_at, &d_at);
    }

    // Order-m pole: cancel (z-z0)^m from the denominator via synthetic division.
    let dp = match PolyExpr::from_expr(&den, z) {
        Some(p) => p,
        None => return noun(),
    };
    let mut r = dp;
    for _ in 0..m {
        r = r.divide_linear(z0);
    }
    let r_expr = r.to_expr(z);
    if r_expr == Expr::int(0) {
        return noun();
    }
    // h(z) = N(z) / R(z), then take the (m-1)th derivative and evaluate at z0.
    let mut h = simplify(&Expr::div(num.clone(), r_expr));
    for _ in 0..(m - 1) {
        h = diff_once(&h, z);
    }
    let val = eval_at(&h, z, z0, env);
    let fact = factorial(m as i64 - 1);
    complex_div(&val, &Expr::int(fact))
}

/// Evaluate `e` at z = z0, expanding first so that powers of complex points
/// like (-%i)^2 fold to -1 (the %i power rule alone only handles base %i).
fn eval_at(e: &Expr, z: &Expr, z0: &Expr, env: &mut crate::env::Environment) -> Expr {
    meval(&expand(&subst(z0, z, e)), env)
}

/// Order of the zero of `den` at `z0`: smallest k with den^(k)(z0) != 0.
/// Returns 0 if den(z0) != 0 (no pole). None if it cannot be determined.
fn pole_order(den: &Expr, z: &Expr, z0: &Expr, env: &mut crate::env::Environment) -> Option<usize> {
    let mut d = den.clone();
    for k in 0..=MAX_POLE_ORDER {
        let val = eval_at(&d, z, z0, env);
        if val != Expr::int(0) {
            return Some(k);
        }
        d = diff_once(&d, z);
    }
    None // order exceeds cap — give up rather than loop
}

fn factorial(n: i64) -> i64 {
    (1..=n).product::<i64>().max(1)
}
