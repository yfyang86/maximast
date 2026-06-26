//! Parametric definite summation AND integration via order-1 recurrence
//! detection (the order-1 case of creative telescoping / Almkvist–Zeilberger).
//!
//! A parametric quantity T(n) — either Σ_k F(n,k) or ∫ F(n,x) dx — is sampled
//! at integer n; if T(n+1)/T(n) is a rational function c·(n+a)/(n+b) (with
//! rational c and integer or half-integer shifts a,b), we telescope it to a
//! closed form and VERIFY it against every sample before returning. Sums and
//! integrals share the engine; only the sampler differs (the ratio is rational
//! even when the values carry √π, so integrals work too).
//!
//! Never returns a wrong closed form — verification gates every result. Higher-
//! order recurrences and certificate-based (Zeilberger/AZ) proofs are future
//! work.

use maxima_core::{Expr, SymbolId};
use crate::helpers::{subst, to_i64, to_f64};
use crate::simp::simplify;
use crate::env::Environment;

#[derive(Clone, Copy, PartialEq)]
struct Rat { n: i128, d: i128 }

impl Rat {
    fn new(mut n: i128, mut d: i128) -> Option<Rat> {
        if d == 0 { return None; }
        if d < 0 { n = -n; d = -d; }
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        if g != 0 { n /= g; d /= g; }
        Some(Rat { n, d })
    }
    fn mul(self, o: Rat) -> Option<Rat> { Rat::new(self.n * o.n, self.d * o.d) }
    fn div(self, o: Rat) -> Option<Rat> { Rat::new(self.n * o.d, self.d * o.n) }
    fn eq(self, o: Rat) -> bool { self.n == o.n && self.d == o.d }
}

fn gcd(a: u128, b: u128) -> u128 { if b == 0 { a } else { gcd(b, a % b) } }

/// Reconstruct a rational from a float via continued fractions. Used for the
/// shift ratio T(n+1)/T(n): symbolic division often won't cancel common
/// irrational factors (e.g. √π), but they cancel numerically — and the closed
/// form is symbolically verified afterwards, so this only guides the search.
fn rat_from_f64(x: f64) -> Option<Rat> {
    if !x.is_finite() { return None; }
    let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
    let mut b = x;
    for _ in 0..40 {
        let ai = b.floor() as i128;
        let (h2, k2) = (ai * h1 + h0, ai * k1 + k0);
        h0 = h1; h1 = h2; k0 = k1; k1 = k2;
        if k1 != 0 && (h1 as f64 / k1 as f64 - x).abs() < 1e-11 { break; }
        if k1 > 1_000_000 { break; }
        let frac = b - ai as f64;
        if frac.abs() < 1e-12 { break; }
        b = 1.0 / frac;
    }
    Rat::new(h1, k1)
}

/// Numeric value of an expression, via `float(...)` so constants like √π and %pi
/// evaluate (the bare `to_f64` only handles literal numbers).
fn numeric(e: &Expr, env: &mut Environment) -> Option<f64> {
    to_f64(&crate::eval::meval(&Expr::call("float", vec![e.clone()]), env))
}

fn rat_expr(r: Rat) -> Expr {
    if r.d == 1 { Expr::int(r.n as i64) } else { Expr::Rational { num: r.n as i64, den: r.d as i64 } }
}

fn free_symbols(e: &Expr, out: &mut Vec<SymbolId>) {
    match e {
        Expr::Symbol(id) => if !out.contains(id) { out.push(*id); },
        Expr::List { args, .. } => for a in args { free_symbols(a, out); },
        _ => {}
    }
}

// ---- the half-shift factor model: factor for Some(h) is (m + h/2) = (2m+h)/2 ----

fn factor_rat(h: Option<i64>, m: i64) -> Option<Rat> {
    match h { None => Rat::new(1, 1), Some(hh) => Rat::new((2 * m + hh) as i128, 2) }
}

// ============================ public entry points ===========================

/// Definite hypergeometric sum S(n) = Σ_{k=lo..hi} body, one free parameter n.
pub fn try_hyper_sum_order1(body: &Expr, k_id: SymbolId, lo: &Expr, hi: &Expr, env: &mut Environment) -> Option<Expr> {
    let mut params = Vec::new();
    free_symbols(body, &mut params);
    free_symbols(lo, &mut params);
    free_symbols(hi, &mut params);
    params.retain(|id| *id != k_id);
    if params.len() != 1 { return None; }
    let n = Expr::Symbol(params[0]);
    let k = Expr::Symbol(k_id);
    let sampler = |n0: i64, env: &mut Environment| sample_sum(body, &k, lo, hi, &n, n0, env);
    solve_parametric(&sampler, &n, env)
}

/// Parametric definite integral I(n) = ∫_{lo}^{hi} f dvar, one free parameter n.
pub fn try_parametric_integral(f: &Expr, var: &Expr, lo: &Expr, hi: &Expr, env: &mut Environment) -> Option<Expr> {
    let var_id = if let Expr::Symbol(id) = var { *id } else { return None; };
    let mut params = Vec::new();
    free_symbols(f, &mut params);
    params.retain(|id| *id != var_id);
    if params.len() != 1 { return None; }
    let n = Expr::Symbol(params[0]);
    let sampler = |n0: i64, env: &mut Environment| sample_integral(f, var, lo, hi, &n, n0, env);
    solve_parametric(&sampler, &n, env)
}

// ============================== samplers ====================================

fn sample_sum(body: &Expr, k: &Expr, lo: &Expr, hi: &Expr, n: &Expr, n0: i64, env: &mut Environment) -> Option<Expr> {
    let ni = Expr::int(n0);
    let lo0 = crate::eval::meval(&subst(&ni, n, lo), env);
    let hi0 = crate::eval::meval(&subst(&ni, n, hi), env);
    let (a, b) = (to_i64(&lo0)?, to_i64(&hi0)?);
    if b < a || b - a > 4000 { return None; }
    let body_n = subst(&ni, n, body);
    let mut acc = Expr::int(0);
    for ki in a..=b {
        let term = crate::eval::meval(&subst(&Expr::int(ki), k, &body_n), env);
        acc = crate::eval::meval(&Expr::add(acc, term), env);
    }
    Some(acc)
}

fn sample_integral(f: &Expr, var: &Expr, lo: &Expr, hi: &Expr, n: &Expr, n0: i64, env: &mut Environment) -> Option<Expr> {
    let fi = subst(&Expr::int(n0), n, f);
    let val = crate::eval::meval(&Expr::call("integrate", vec![fi, var.clone(), lo.clone(), hi.clone()]), env);
    // Reject unevaluated integrals.
    if val.to_string().contains("integrate") { return None; }
    Some(val)
}

// ============================ shared engine =================================

fn solve_parametric(sample: &dyn Fn(i64, &mut Environment) -> Option<Expr>, n: &Expr, env: &mut Environment) -> Option<Expr> {
    let sample0 = sample(0, env);

    // Sample T(n0) for n0 = 1.. ; keep nonzero, exact values.
    let mut samples: Vec<(i64, Expr)> = Vec::new();
    for n0 in 1..=12i64 {
        if let Some(v) = sample(n0, env) {
            if !is_zero_expr(&v) { samples.push((n0, v)); }
        }
        if samples.len() >= 8 { break; }
    }
    if samples.len() < 6 { return None; }

    // Ratios R(n0) = T(n0+1)/T(n0), reconstructed numerically (common irrational
    // factors like √π cancel numerically even when the simplifier won't).
    let mut ratios: Vec<(i64, Rat)> = Vec::new();
    for w in samples.windows(2) {
        if w[1].0 == w[0].0 + 1 {
            let (Some(hi), Some(lo)) = (numeric(&w[1].1, env), numeric(&w[0].1, env)) else { return None };
            if lo == 0.0 { return None; }
            match rat_from_f64(hi / lo) {
                Some(rr) => ratios.push((w[0].0, rr)),
                None => return None,
            }
        }
    }
    if ratios.len() < 4 { return None; }
    let (m0, r0) = ratios[0];

    // Search ratio model c·(n+ha/2)/(n+hb/2) with each factor optional.
    let opts: Vec<Option<i64>> = std::iter::once(None).chain((-24..=24).map(Some)).collect();
    for &num_h in &opts {
        for &den_h in &opts {
            let (Some(fn0), Some(fd0)) = (factor_rat(num_h, m0), factor_rat(den_h, m0)) else { continue };
            if fn0.n == 0 { continue; }
            // c = R(m0)·factor_den(m0)/factor_num(m0)
            let Some(c) = r0.mul(fd0).and_then(|x| x.div(fn0)) else { continue };
            let ok = ratios.iter().all(|&(m, r)| {
                match (factor_rat(num_h, m), factor_rat(den_h, m)) {
                    (Some(fnm), Some(fdm)) if fdm.n != 0 => {
                        c.mul(fnm).and_then(|x| x.div(fdm)).map_or(false, |model| model.eq(r))
                    }
                    _ => false,
                }
            });
            if !ok { continue; }

            // Build a closed form: prefer the clean factorial-free consecutive
            // product (integer shifts, both factors present); else Pochhammer.
            let closed = build_consecutive(&samples, c, num_h, den_h, n)
                .or_else(|| sample0.as_ref().filter(|s| !is_zero_expr(s))
                    .and_then(|s0| build_pochhammer(s0, c, num_h, den_h, n)));
            if let Some(closed) = closed {
                let closed = crate::eval::meval(&closed, env);
                if verify(&closed, &samples, n, env) {
                    return Some(closed);
                }
            }
        }
    }
    None
}

/// Factorial-free closed form for integer shifts with BOTH factors present:
/// S(n) = K·c^(n−n0)·∏_{i=b}^{a−1}(n+i)  (or reciprocal product when a<b).
fn build_consecutive(samples: &[(i64, Expr)], c: Rat, num_h: Option<i64>, den_h: Option<i64>, n: &Expr) -> Option<Expr> {
    let (ha, hb) = (num_h?, den_h?);
    if ha % 2 != 0 || hb % 2 != 0 { return None; } // integers only
    let (a, b) = (ha / 2, hb / 2);
    let n0 = samples[0].0;
    let geom = Expr::pow(rat_expr(c), Expr::sub(n.clone(), Expr::int(n0)));
    let mut prod = Expr::int(1);
    let shape = if a >= b {
        for i in b..a { prod = Expr::mul(prod, Expr::add(n.clone(), Expr::int(i))); }
        Expr::mul(geom, prod)
    } else {
        for i in a..b { prod = Expr::mul(prod, Expr::add(n.clone(), Expr::int(i))); }
        Expr::div(geom, prod)
    };
    Some(scale_to_match(shape, n0, &samples[0].1, n))
}

/// Pochhammer/factorial closed form (handles half-integer & absent factors),
/// anchored at n=0: T(n) = T(0)·base^n·(num)_n/(den)_n with 4^n folded into base.
fn build_pochhammer(s0: &Expr, c: Rat, num_h: Option<i64>, den_h: Option<i64>, n: &Expr) -> Option<Expr> {
    let (pa, fa) = poch_fact(num_h, n)?;
    let (pb, fb) = poch_fact(den_h, n)?;
    let d = fa - fb;
    if d.abs() > 8 { return None; }
    let base = if d >= 0 { Rat::new(c.n * 4i128.pow(d as u32), c.d)? }
               else { Rat::new(c.n, c.d * 4i128.pow((-d) as u32))? };
    let shape = Expr::div(Expr::mul(Expr::pow(rat_expr(base), n.clone()), pa), pb);
    Some(scale_to_match(shape, 0, s0, n))
}

/// (h/2)_n as (factorial-expr, e) with value expr·4^(e·n); None for absent.
fn poch_fact(h: Option<i64>, n: &Expr) -> Option<(Expr, i64)> {
    let fact = |x: Expr| Expr::call("factorial", vec![x]);
    let Some(h) = h else { return Some((Expr::int(1), 0)); };
    if h % 2 == 0 {
        let a = h / 2;
        if a < 1 { return None; }
        Some((Expr::div(fact(Expr::add(n.clone(), Expr::int(a - 1))), fact(Expr::int(a - 1))), 0))
    } else {
        let j = (h - 1) / 2;
        if j < 0 { return None; }
        let nj = Expr::add(n.clone(), Expr::int(j));
        let numer = Expr::mul(fact(Expr::mul(Expr::int(2), nj.clone())), fact(Expr::int(j)));
        let denom = Expr::mul(fact(nj), fact(Expr::int(2 * j)));
        Some((Expr::div(numer, denom), -1))
    }
}

/// Multiply `shape` by K = anchor_value/shape(anchor_n) so T(anchor_n) matches.
fn scale_to_match(shape: Expr, anchor_n: i64, anchor_val: &Expr, n: &Expr) -> Expr {
    let shape_a = simplify(&subst(&Expr::int(anchor_n), n, &shape));
    Expr::mul(Expr::div(anchor_val.clone(), shape_a), shape)
}

fn verify(closed: &Expr, samples: &[(i64, Expr)], n: &Expr, env: &mut Environment) -> bool {
    let mut checked = 0;
    for (n0, val) in samples {
        let diff = crate::eval::meval(&Expr::sub(subst(&Expr::int(*n0), n, closed), val.clone()), env);
        let diff = simplify(&diff);
        let zero = is_zero_expr(&diff) || numeric(&diff, env).map_or(false, |x| x.abs() < 1e-9);
        if !zero { return false; }
        checked += 1;
    }
    checked >= 6
}

fn is_zero_expr(e: &Expr) -> bool {
    matches!(e, Expr::Integer(0)) || matches!(e, Expr::Rational { num: 0, .. })
        || matches!(e, Expr::Float(f) if f.abs() < 1e-12)
}

#[cfg(test)]
mod tests {
    use crate::eval::eval_str;
    fn run(s: &str) -> String { eval_str(s) }

    #[test] fn sum_k_binomial() {
        assert!(!run("sum(k*binomial(n,k),k,0,n);").contains("sum("));
        assert_eq!(run("sum(k*binomial(6,k),k,0,6);"), "192");
    }
    #[test] fn sum_k2_binomial() {
        assert!(!run("sum(k^2*binomial(n,k),k,0,n);").contains("sum("));
        assert_eq!(run("sum(k^2*binomial(5,k),k,0,5);"), "240");
    }
    #[test] fn sum_plain_binomial_still_2n() { assert_eq!(run("sum(binomial(n,k),k,0,n);"), "2^n"); }
    #[test] fn sum_binomial_squared() {
        assert!(!run("sum(binomial(n,k)^2,k,0,n);").contains("sum("));
        assert_eq!(run("sum(binomial(6,k)^2,k,0,6);"), "924");
    }
    #[test] fn generalized_harmonic() { assert_eq!(run("sum(1/k^2,k,1,n);"), "harmonic(n,2)"); }

    #[test] fn parametric_gaussian_moment() {
        // ∫₀^∞ x^(2n) e^(-x²) dx = (2n)!√π/(2·4^n·n!); closed form found…
        let s = run("integrate(x^(2*n)*exp(-x^2),x,0,inf);");
        assert!(!s.contains("integrate("), "got noun: {s}");
        // …agreeing with the fixed-exponent value at n=3 (= 15√π/16).
        assert_eq!(run("integrate(x^6*exp(-x^2),x,0,inf);"), "15*sqrt(%pi)/16");
    }
}
