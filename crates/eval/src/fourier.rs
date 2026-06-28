//! Fourier transform F(ω) = ∫_{−∞}^{∞} f(x)·e^{−iωx} dx, V13 3e. A small table
//! of the canonical pairs (Gaussian, two-sided exponential, Lorentzian) plus
//! linearity and constant factoring; the rational case is otherwise left to the
//! residue integrals (`integrate(f·cos(ωx), …)`). correct-or-noun.

use maxima_core::{Expr, Operator, resolve};
use crate::simp::simplify;
use crate::helpers::{contains_var, to_f64};

pub(crate) fn eval_fourier(name: &str, args: &[Expr], _env: &mut crate::env::Environment) -> Option<Expr> {
    if name != "fourier_transform" && name != "fourier" { return None; }
    if args.len() != 3 { return None; }
    let (f, x, w) = (&args[0], &args[1], &args[2]);
    if !matches!(x, Expr::Symbol(_)) || !matches!(w, Expr::Symbol(_)) { return None; }
    Some(transform(f, x, w))
}

fn transform(f: &Expr, x: &Expr, w: &Expr) -> Expr {
    // Linearity.
    if let Expr::List { op: Operator::MPlus, args, .. } = f {
        let parts: Vec<Expr> = args.iter().map(|a| transform(a, x, w)).collect();
        if parts.iter().any(is_noun) {
            return Expr::call("fourier_transform", vec![f.clone(), x.clone(), w.clone()]);
        }
        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: parts });
    }
    // Constant factor.
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        let (consts, dep): (Vec<&Expr>, Vec<&Expr>) = args.iter().partition(|a| !contains_var(a, x));
        if !consts.is_empty() && !dep.is_empty() {
            let c = if consts.len() == 1 { consts[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
            let inner = if dep.len() == 1 { dep[0].clone() }
                else { Expr::List { op: Operator::MTimes, simplified: false, args: dep.into_iter().cloned().collect() } };
            let ft = transform(&inner, x, w);
            if !is_noun(&ft) { return simplify(&Expr::mul(c, ft)); }
        }
    }
    table(f, x, w)
        .or_else(|| rational_transform(f, x, w))
        .unwrap_or_else(||
            Expr::call("fourier_transform", vec![f.clone(), x.clone(), w.clone()]))
}

/// Rational P/Q (strictly proper, Q with simple irreducible quadratic factors):
/// F(ω) = C(ω) − i·S(ω) where C,S are the Fourier cos/sin integrals. Combining
/// the per-quadratic closed forms (pole α±iω_q, numerator Bx+C) collapses to
///   F(ω) = Σ (π/ω_q)·e^(−ω·ω_q)·[(Bα+C) − i·B·ω_q]·e^(−iωα)   (ω>0).
/// Verified: F{1/(x²+1)}=π·e^(−ω), F{x/(x²+1)}=−iπ·e^(−ω). Assumes ω>0.
fn rational_transform(f: &Expr, x: &Expr, w: &Expr) -> Option<Expr> {
    use num::{BigRational, BigInt, Zero};
    let Expr::Symbol(var_id) = x else { return None };
    let terms = crate::laplace::partial_fraction_terms(f, *var_id)?;
    let four = BigRational::from(BigInt::from(4));
    let two = BigRational::from(BigInt::from(2));
    let pi = Expr::sym("%pi");
    let i = Expr::sym("%i");
    let br = crate::helpers::bigrat_to_expr;
    let mut result = Expr::int(0);
    for (q, j, ncoef) in terms {
        if q.len() != 3 || j != 1 { return None; }            // real/repeated pole → noun
        let (b, c) = (q[1].clone(), q[0].clone());
        if &(&b * &b) - &(&four * &c) >= BigRational::zero() { return None; } // real roots
        let alpha = -(&b / &two);
        let omega2 = &c - &(&(&b * &b) / &four);
        let bb = ncoef.get(1).cloned().unwrap_or_else(BigRational::zero);
        let cc = ncoef[0].clone();
        let bac = &(&bb * &alpha) + &cc;                       // Bα + C
        let omega = Expr::call("sqrt", vec![br(&omega2)]);     // ω_q
        // amplitude (Bα+C) − i·B·ω_q
        let amp = Expr::sub(br(&bac),
            Expr::mul(i.clone(), Expr::mul(br(&bb), omega.clone())));
        // phase e^(−iωα)
        let phase = Expr::call("exp", vec![simplify(&Expr::neg(
            Expr::mul(i.clone(), Expr::mul(w.clone(), br(&alpha)))))]);
        // damping e^(−ω·ω_q)
        let damp = Expr::call("exp", vec![simplify(&Expr::neg(
            Expr::mul(w.clone(), omega.clone())))]);
        let term = Expr::mul(Expr::div(pi.clone(), omega),
            Expr::mul(damp, Expr::mul(amp, phase)));
        result = simplify(&Expr::add(result, term));
    }
    Some(meval_fresh(&result))
}

fn meval_fresh(e: &Expr) -> Expr {
    crate::eval::meval(e, &mut crate::env::Environment::new())
}

fn table(f: &Expr, x: &Expr, w: &Expr) -> Option<Expr> {
    // F{exp(−a·x²)} = √(π/a)·exp(−ω²/(4a))   (a>0)
    if let Some(a) = gaussian_coeff(f, x) {
        if to_f64(&a).map(|v| v > 0.0).unwrap_or(true) {
            return Some(simplify(&Expr::mul(
                Expr::call("sqrt", vec![Expr::div(Expr::sym("%pi"), a.clone())]),
                Expr::call("exp", vec![simplify(&Expr::neg(
                    Expr::div(Expr::pow(w.clone(), Expr::int(2)), Expr::mul(Expr::int(4), a))))]))));
        }
    }
    // F{exp(−a·|x|)} = 2a/(a²+ω²)   (a>0)
    if let Some(a) = abs_exp_coeff(f, x) {
        return Some(simplify(&Expr::div(
            Expr::mul(Expr::int(2), a.clone()),
            Expr::add(Expr::pow(a, Expr::int(2)), Expr::pow(w.clone(), Expr::int(2))))));
    }
    // F{1/(x²+a²)} = (π/a)·exp(−a·|ω|)   (a>0)
    if let Some(a2) = lorentzian_coeff(f, x) {
        let a = simplify(&Expr::call("sqrt", vec![a2]));
        return Some(simplify(&Expr::mul(
            Expr::div(Expr::sym("%pi"), a.clone()),
            Expr::call("exp", vec![simplify(&Expr::neg(
                Expr::mul(a, Expr::call("abs", vec![w.clone()]))))]))));
    }
    None
}

/// a from exp(−a·x²) (a free of x).
fn gaussian_coeff(f: &Expr, x: &Expr) -> Option<Expr> {
    let Expr::List { op: Operator::Named(id), args, .. } = f else { return None };
    if resolve(*id) != "exp" || args.len() != 1 { return None; }
    // arg = −a·x²  ⇒  a = −arg / x²
    let x2 = Expr::pow(x.clone(), Expr::int(2));
    let a = simplify(&crate::eval::ratsimp_pub(&Expr::div(Expr::neg(args[0].clone()), x2)));
    if contains_var(&a, x) { None } else { Some(a) }
}

/// a from exp(−a·|x|) (a free of x).
fn abs_exp_coeff(f: &Expr, x: &Expr) -> Option<Expr> {
    let Expr::List { op: Operator::Named(id), args, .. } = f else { return None };
    if resolve(*id) != "exp" || args.len() != 1 { return None; }
    let absx = Expr::call("abs", vec![x.clone()]);
    let a = simplify(&Expr::div(Expr::neg(args[0].clone()), absx));
    if contains_var(&a, x) { None } else { Some(a) }
}

/// a² from 1/(x²+a²) (a² free of x, positive).
fn lorentzian_coeff(f: &Expr, x: &Expr) -> Option<Expr> {
    let Expr::List { op: Operator::MExpt, args, .. } = f else { return None };
    if args.len() != 2 || args[1] != Expr::int(-1) { return None; }
    let Expr::List { op: Operator::MPlus, args: ta, .. } = &args[0] else { return None };
    if ta.len() != 2 { return None; }
    let x2 = Expr::pow(x.clone(), Expr::int(2));
    let (a2, has_x2) = if ta[0] == x2 { (ta[1].clone(), true) }
        else if ta[1] == x2 { (ta[0].clone(), true) } else { (Expr::int(0), false) };
    if !has_x2 || contains_var(&a2, x) { return None; }
    if to_f64(&a2).map(|v| v <= 0.0).unwrap_or(false) { return None; }
    Some(a2)
}

fn is_noun(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), .. } if resolve(*id) == "fourier_transform")
}
