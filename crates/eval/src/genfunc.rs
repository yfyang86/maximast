//! Generating-function summation: Σ_{k=lo}^∞ p(k)·xᵏ for a polynomial p and a
//! free base x, giving a rational function of x. Built from the geometric
//! series Σ xᵏ = 1/(1−x) by applying the operator (x·d/dx) once per power of k
//! (so Σ k·xᵏ = x/(1−x)², Σ k²·xᵏ = x(1+x)/(1−x)³, …). The closed form is
//! numerically verified against a partial sum at x = 1/3 before it is returned
//! (correct-or-noun: a divergent or mis-derived form falls back to a noun).

use maxima_core::{Expr, Operator, SymbolId, intern};
use crate::eval::{expand, meval, diff_once};
use crate::simp::simplify;
use crate::env::Environment;
use crate::helpers::{to_f64, subst};

/// Σ_{k=lo}^∞ body, when body = p(k)·xᵏ with x a free symbol ≠ k and p a
/// polynomial in k. lo must be 0 or 1. None if the shape doesn't match or the
/// numeric check fails.
pub(crate) fn geometric_poly_gf(body: &Expr, var: SymbolId, lo: i64, env: &mut Environment) -> Option<Expr> {
    if lo != 0 && lo != 1 { return None; }
    let (xbase, poly) = split_base_poly(body, var)?;
    // The base is either a free symbol (≠ the summation variable) → symbolic GF,
    // or a number with |base| < 1 → convergent numeric series (derive with a
    // dummy symbol, then substitute). Anything else (|base| ≥ 1, var-dependent
    // base) declines.
    let (x_id, numeric_base) = match &xbase {
        Expr::Symbol(id) if *id != var => (*id, false),
        Expr::Integer(_) | Expr::Rational { .. } | Expr::Float(_) => {
            if to_f64(&xbase)?.abs() >= 1.0 { return None; }
            (intern("%gfx"), true)
        }
        _ => return None,
    };
    if contains_sym(&poly, x_id) { return None; }
    let x = Expr::Symbol(x_id);

    let p = maxima_poly::expr_to_poly(&expand(&poly), var)?;
    let deg = p.degree()? as usize;

    // g_j = (x d/dx)^j (1/(1-x)), so Σ_{k≥0} k^j x^k = g_j.
    let one_minus_x = simplify(&Expr::sub(Expr::int(1), x.clone()));
    let mut g = simplify(&Expr::div(Expr::int(1), one_minus_x)); // 1/(1-x)
    let mut gj = vec![g.clone()];
    for _ in 0..deg {
        g = simplify(&Expr::mul(x.clone(), diff_once(&g, &x)));
        gj.push(simplify(&g));
    }

    // Σ_{k≥0} p(k) x^k = Σ_j c_j g_j.
    let mut total = Expr::int(0);
    for j in 0..=deg {
        let c = coeff_expr(&p, j as u32);
        if c == Expr::int(0) { continue; }
        total = simplify(&Expr::add(total, simplify(&Expr::mul(c, gj[j].clone()))));
    }
    // lo = 1 drops the k=0 term p(0)·x^0 = p(0).
    if lo == 1 {
        let p0 = coeff_expr(&p, 0);
        total = simplify(&Expr::sub(total, p0));
    }
    // For a numeric base, substitute it back and evaluate to a number; the
    // verification then needs no x assignment (the body already holds the base).
    let (result, xset) = if numeric_base {
        (meval(&subst(&xbase, &x, &total), env), None)
    } else {
        (meval(&total, env), Some((x_id, 1.0 / 3.0)))
    };
    if numeric_check(body, var, lo, &result, xset, env) { Some(result) } else { None }
}

/// Numeric value of an expression, evaluating powers/functions as f64 (so e.g.
/// (1/2)^200 becomes a tiny float rather than overflowing an integer power).
fn nval(e: &Expr) -> Option<f64> {
    to_f64(&crate::helpers::expr_to_float(e))
}

/// Split body into (base, poly) where body = poly·base^var. Handles base^var
/// alone (poly = 1) and a product with one such power factor.
fn split_base_poly(body: &Expr, var: SymbolId) -> Option<(Expr, Expr)> {
    let var_e = Expr::Symbol(var);
    let is_xk = |e: &Expr| -> Option<Expr> {
        if let Expr::List { op: Operator::MExpt, args, .. } = e {
            if args.len() == 2 && args[1] == var_e { return Some(args[0].clone()); }
        }
        None
    };
    if let Some(base) = is_xk(body) {
        return Some((base, Expr::int(1)));
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = body {
        let mut base = None;
        let mut rest: Vec<Expr> = Vec::new();
        for a in args {
            if base.is_none() {
                if let Some(b) = is_xk(a) { base = Some(b); continue; }
            }
            rest.push(a.clone());
        }
        let base = base?;
        let poly = if rest.is_empty() { Expr::int(1) }
            else if rest.len() == 1 { rest.pop().unwrap() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: rest } };
        return Some((base, poly));
    }
    None
}

fn coeff_expr(p: &maxima_poly::Poly, e: u32) -> Expr {
    let c = p.terms.iter().find(|(pe, _)| *pe == e).map(|(_, c)| c.clone())
        .unwrap_or_else(maxima_poly::Coeff::zero);
    match c {
        maxima_poly::Coeff::Int(n) => Expr::int(n),
        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
    }
}

fn contains_sym(e: &Expr, id: SymbolId) -> bool {
    match e {
        Expr::Symbol(s) => *s == id,
        Expr::List { args, .. } => args.iter().any(|a| contains_sym(a, id)),
        _ => false,
    }
}

/// Verify the closed form against a partial sum of the series. For a symbolic
/// base, `xset` assigns x = 1/3 in both the body and the result; for a numeric
/// base, the body already holds the value and the result is a plain number.
fn numeric_check(body: &Expr, var: SymbolId, lo: i64, result: &Expr,
                 xset: Option<(SymbolId, f64)>, env: &mut Environment) -> bool {
    let mut partial = 0.0f64;
    env.push_scope();
    if let Some((xid, xv)) = xset { env.set_local(xid, Expr::Float(xv)); }
    for k in lo..=120 {
        env.set_local(var, Expr::int(k));
        if let Some(t) = nval(&meval(body, env)) { partial += t; } else { env.pop_scope(); return false; }
    }
    let closed = nval(&meval(result, env));
    env.pop_scope();
    match closed {
        Some(c) => (c - partial).abs() < 1e-6 * (1.0 + c.abs()),
        None => false,
    }
}
