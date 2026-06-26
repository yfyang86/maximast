//! Numeric solvers (V13 3d): root-finding, quadrature, ODE integration.
//! All evaluate the expression numerically by substituting a float for the
//! variable and reducing via `meval`. f64 precision (arbitrary precision would
//! follow a real bigfloat backend).

use maxima_core::{Expr, Operator};
use crate::env::Environment;
use crate::eval::meval;
use crate::helpers::{subst, to_f64};

/// Numeric value of f at var = val.
fn nf(f: &Expr, var: &Expr, val: f64, env: &mut Environment) -> Option<f64> {
    to_f64(&meval(&subst(&Expr::Float(val), var, f), env))
}

/// expr, or lhs−rhs if expr is an equation.
fn as_zero_form(expr: &Expr) -> Expr {
    if let Expr::List { op: Operator::MEqual, args, .. } = expr {
        if args.len() == 2 {
            return Expr::sub(args[0].clone(), args[1].clone());
        }
    }
    expr.clone()
}

/// find_root(expr, x, a, b): a root of expr in [a,b] by bisection (needs a sign
/// change). Returns the root as a Float, or None.
fn find_root(f: &Expr, var: &Expr, a: f64, b: f64, env: &mut Environment) -> Option<f64> {
    let f = as_zero_form(f);
    let (mut lo, mut hi) = (a.min(b), a.max(b));
    let mut flo = nf(&f, var, lo, env)?;
    let mut fhi = nf(&f, var, hi, env)?;
    if flo == 0.0 { return Some(lo); }
    if fhi == 0.0 { return Some(hi); }
    if flo * fhi > 0.0 { return None; } // no bracketed sign change
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let fmid = nf(&f, var, mid, env)?;
        if fmid == 0.0 || (hi - lo).abs() < 1e-15 { return Some(mid); }
        if flo * fmid < 0.0 { hi = mid; fhi = fmid; } else { lo = mid; flo = fmid; }
        let _ = fhi;
    }
    Some(0.5 * (lo + hi))
}

/// Romberg integration of f over [a,b].
fn romberg(f: &Expr, var: &Expr, a: f64, b: f64, env: &mut Environment) -> Option<f64> {
    const K: usize = 18;
    let mut r = vec![vec![0.0f64; K]; K];
    let mut h = b - a;
    r[0][0] = 0.5 * h * (nf(f, var, a, env)? + nf(f, var, b, env)?);
    for k in 1..K {
        h *= 0.5;
        let n = 1usize << (k - 1);
        let mut sum = 0.0;
        for i in 1..=n {
            sum += nf(f, var, a + (2 * i - 1) as f64 * h, env)?;
        }
        r[k][0] = 0.5 * r[k - 1][0] + h * sum;
        for j in 1..=k {
            let p = 4f64.powi(j as i32);
            r[k][j] = (p * r[k][j - 1] - r[k - 1][j - 1]) / (p - 1.0);
        }
        if k >= 5 && (r[k][k] - r[k - 1][k - 1]).abs() < 1e-12 {
            return Some(r[k][k]);
        }
    }
    Some(r[K - 1][K - 1])
}

/// Classical RK4 for dy/dt = rhs(t,y): rk(rhs, y, y0, [t, t0, tf, h]) → list of
/// [t, y] points (matches Maxima's single-equation rk).
fn rk(rhs: &Expr, yvar: &Expr, tvar: &Expr, y0: f64, t0: f64, tf: f64, h: f64,
      env: &mut Environment) -> Option<Expr> {
    if h <= 0.0 || tf < t0 { return None; }
    let g = |t: f64, y: f64, env: &mut Environment| -> Option<f64> {
        let e = subst(&Expr::Float(t), tvar, &subst(&Expr::Float(y), yvar, rhs));
        to_f64(&meval(&e, env))
    };
    let mut t = t0;
    let mut y = y0;
    let mut pts = vec![Expr::list(vec![Expr::Float(t), Expr::Float(y)])];
    let steps = ((tf - t0) / h).ceil() as usize;
    for _ in 0..steps.min(1_000_000) {
        let k1 = g(t, y, env)?;
        let k2 = g(t + h / 2.0, y + h / 2.0 * k1, env)?;
        let k3 = g(t + h / 2.0, y + h / 2.0 * k2, env)?;
        let k4 = g(t + h, y + h * k3, env)?;
        y += h / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
        t += h;
        if t > tf + 1e-12 { break; }
        pts.push(Expr::list(vec![Expr::Float(t), Expr::Float(y)]));
    }
    Some(Expr::list(pts))
}

/// Numeric value of a bound, evaluating symbolic constants (float(%pi) = 3.14…).
fn numf(e: &Expr, env: &mut Environment) -> Option<f64> {
    to_f64(&meval(&Expr::call("float", vec![e.clone()]), env))
}

/// Dispatch for the numeric builtins. Returns None to fall through to a noun.
pub fn eval_numeric_func(name: &str, args: &[Expr], env: &mut Environment) -> Option<Expr> {
    match name {
        "find_root" | "newton" if args.len() >= 4 => {
            let (a, b) = (numf(&args[2], env)?, numf(&args[3], env)?);
            find_root(&args[0], &args[1], a, b, env).map(Expr::Float)
        }
        "romberg" | "quad_qags" | "quad_qag" if args.len() >= 4 => {
            let (a, b) = (numf(&args[2], env)?, numf(&args[3], env)?);
            romberg(&args[0], &args[1], a, b, env).map(Expr::Float)
        }
        "rk" if args.len() == 4 => {
            // rk(rhs, y, y0, [t, t0, tf, h])
            let y0 = numf(&args[2], env)?;
            if let Expr::List { op: Operator::MList, args: rng, .. } = &args[3] {
                if rng.len() == 4 {
                    let (t0, tf, h) = (numf(&rng[1], env)?, numf(&rng[2], env)?, numf(&rng[3], env)?);
                    return rk(&args[0], &args[1], &rng[0], y0, t0, tf, h, env);
                }
            }
            None
        }
        _ => None,
    }
}
