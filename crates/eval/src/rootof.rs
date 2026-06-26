//! RootOf: implicit algebraic roots of a univariate polynomial that has no
//! radical solution (e.g. a general quintic). `solve` returns `rootof(p,x,k)`
//! nouns indexed by a deterministic ordering of all roots; `float`/`bfloat`
//! evaluate them numerically (all roots via Durand–Kerner, then the k-th in
//! order; real roots refine to full bigfloat precision via Newton).

use maxima_core::{Expr, Operator, BigFloatVal};
use num::complex::Complex64;
use astro_float::{BigFloat, Consts, RoundingMode, Radix};

const RM: RoundingMode = RoundingMode::ToEven;

/// Dense monic complex coefficients [a0, a1, …, 1] of a polynomial, low→high.
fn monic_coeffs(poly: &maxima_poly::Poly) -> Option<Vec<Complex64>> {
    let d = poly.degree()? as usize;
    if d == 0 { return None; }
    let mut c = vec![Complex64::new(0.0, 0.0); d + 1];
    for (e, coeff) in &poly.terms {
        c[*e as usize] = Complex64::new(coeff_f64(coeff), 0.0);
    }
    let lead = c[d];
    if lead.norm() == 0.0 { return None; }
    for ci in c.iter_mut() { *ci /= lead; }
    Some(c)
}

fn coeff_f64(c: &maxima_poly::Coeff) -> f64 {
    match c {
        maxima_poly::Coeff::Int(n) => *n as f64,
        maxima_poly::Coeff::Rat(n, d) => *n as f64 / *d as f64,
    }
}

fn horner(c: &[Complex64], x: Complex64) -> Complex64 {
    let mut acc = Complex64::new(0.0, 0.0);
    for ci in c.iter().rev() { acc = acc * x + ci; }
    acc
}

/// All complex roots via the Durand–Kerner (Weierstrass) iteration, sorted by
/// (re, im) — rounded to stabilise ordering against floating jitter.
pub(crate) fn all_roots(poly: &maxima_poly::Poly) -> Option<Vec<Complex64>> {
    let c = monic_coeffs(poly)?;
    let d = c.len() - 1;
    // Distinct starting points off the real axis: (0.4+0.9i)^k.
    let seed = Complex64::new(0.4, 0.9);
    let mut r: Vec<Complex64> = (0..d).map(|k| seed.powi(k as i32)).collect();
    for _ in 0..500 {
        let mut max_delta = 0.0f64;
        for i in 0..d {
            let mut denom = Complex64::new(1.0, 0.0);
            for j in 0..d {
                if i != j { denom *= r[i] - r[j]; }
            }
            if denom.norm() < 1e-300 { continue; }
            let delta = horner(&c, r[i]) / denom;
            r[i] -= delta;
            max_delta = max_delta.max(delta.norm());
        }
        if max_delta < 1e-14 { break; }
    }
    // Real roots first (ascending), then complex roots by (re, im). Rounding
    // stabilises the order against floating jitter.
    let key = |z: &Complex64| {
        let is_complex = (z.im.abs() >= 1e-9) as i8;
        (is_complex, (z.re * 1e9).round() as i128, (z.im * 1e9).round() as i128)
    };
    r.sort_by(|a, b| key(a).cmp(&key(b)));
    Some(r)
}

/// `[var = rootof(p, var, k), …]` for k = 1..=deg — the structured solution for
/// a numeric univariate polynomial unsolvable by radicals.
pub(crate) fn make_rootof_solutions(poly: &maxima_poly::Poly, var: maxima_core::SymbolId) -> Option<Expr> {
    let d = poly.degree()? as usize;
    if d < 1 { return None; }
    // Only for fully numeric coefficients (root indexing is numeric).
    all_roots(poly)?;
    let p_expr = maxima_poly::poly_to_expr(poly);
    let v = Expr::Symbol(var);
    let eqs = (1..=d).map(|k| Expr::List {
        op: Operator::MEqual,
        simplified: false,
        args: vec![v.clone(), Expr::call("rootof", vec![p_expr.clone(), v.clone(), Expr::int(k as i64)])],
    }).collect();
    Some(Expr::list(eqs))
}

/// Parse a `rootof(p, x, k)` argument list into (poly, k) (1-based index).
fn parse_rootof(args: &[Expr]) -> Option<(maxima_poly::Poly, usize)> {
    if args.len() != 3 { return None; }
    let var = match &args[1] { Expr::Symbol(id) => *id, _ => return None };
    let k = match &args[2] { Expr::Integer(n) if *n >= 1 => *n as usize, _ => return None };
    let poly = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var)?;
    let d = poly.degree()? as usize;
    if k > d { return None; }
    Some((poly, k))
}

/// The k-th root as an f64 Expr — a real `Float`, or `re + im*%i`.
pub(crate) fn eval_rootof_float(args: &[Expr]) -> Option<Expr> {
    let (poly, k) = parse_rootof(args)?;
    let roots = all_roots(&poly)?;
    let z = roots.get(k - 1)?;
    Some(complex_to_expr(z))
}

fn complex_to_expr(z: &Complex64) -> Expr {
    if z.im.abs() < 1e-12 {
        Expr::Float(z.re)
    } else {
        Expr::add(Expr::Float(z.re), Expr::mul(Expr::Float(z.im), Expr::sym("%i")))
    }
}

/// `bfloat(rootof(p,x,k))` at `bits` precision. A real root is refined to full
/// precision by Newton's method in astro-float; a complex root falls back to
/// the f64 value (astro-float is real-only).
pub(crate) fn eval_rootof_bfloat(args: &[Expr], bits: usize, digits: i64) -> Option<Expr> {
    let (poly, k) = parse_rootof(args)?;
    let roots = all_roots(&poly)?;
    let z = roots.get(k - 1)?;
    if z.im.abs() >= 1e-12 {
        return Some(complex_to_expr(z)); // complex: f64 fallback
    }
    // Newton refine the real root to `bits` precision.
    let (p, dp) = bigfloat_poly_and_deriv(&poly, bits);
    let mut x = BigFloat::from_f64(z.re, bits);
    for _ in 0..200 {
        let fx = horner_big(&p, &x, bits);
        let dfx = horner_big(&dp, &x, bits);
        if dfx.is_zero() { break; }
        let step = fx.div(&dfx, bits, RM);
        x = x.sub(&step, bits, RM);
        if step.abs() < BigFloat::from_f64(10f64.powi(-(digits as i32) - 4), bits) { break; }
    }
    let mut cc = Consts::new().ok()?;
    let s = crate::bigfloat::round_sig_pub(&x.format(Radix::Dec, RM, &mut cc).unwrap_or_default(), digits as usize);
    Some(Expr::BigFloat(Box::new(BigFloatVal { digits: s.into_boxed_str(), bits: bits as u32 })))
}

/// (coeffs, derivative-coeffs) of the polynomial as astro-float values, low→high.
fn bigfloat_poly_and_deriv(poly: &maxima_poly::Poly, bits: usize) -> (Vec<BigFloat>, Vec<BigFloat>) {
    let d = poly.degree().unwrap_or(0) as usize;
    let mut p = vec![BigFloat::from_i64(0, bits); d + 1];
    for (e, c) in &poly.terms {
        p[*e as usize] = coeff_big(c, bits);
    }
    let dp: Vec<BigFloat> = (1..=d).map(|i| p[i].mul(&BigFloat::from_i64(i as i64, bits), bits, RM)).collect();
    (p, dp)
}

fn coeff_big(c: &maxima_poly::Coeff, bits: usize) -> BigFloat {
    match c {
        maxima_poly::Coeff::Int(n) => BigFloat::from_i64(*n, bits),
        maxima_poly::Coeff::Rat(n, d) =>
            BigFloat::from_i64(*n, bits).div(&BigFloat::from_i64(*d, bits), bits, RM),
    }
}

fn horner_big(c: &[BigFloat], x: &BigFloat, bits: usize) -> BigFloat {
    let mut acc = BigFloat::from_i64(0, bits);
    for ci in c.iter().rev() {
        acc = acc.mul(x, bits, RM).add(ci, bits, RM);
    }
    acc
}
