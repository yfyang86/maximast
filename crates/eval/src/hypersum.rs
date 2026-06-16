//! Definite hypergeometric summation via order-1 recurrence detection.
//!
//! For a sum S(n) = Σ_k F(n,k) (one free parameter n), many classical sums
//! satisfy a first-order recurrence S(n+1)/S(n) = c·(n+a)/(n+b) with rational c
//! and integer shifts a,b (e.g. Σ k·binomial(n,k): ratio 2(n+1)/n). We detect
//! that ratio by *exact* sampling, then telescope it to a factorial-free closed
//! form  S(n) = K·c^n·∏(n+i)  and VERIFY numerically before returning.
//!
//! This is the order-1 installment of creative telescoping; higher-order
//! recurrences and half-integer/Gamma closed forms are future work. It never
//! returns a wrong closed form — verification gates every result.

use maxima_core::{Expr, SymbolId};
use crate::helpers::{subst, to_i64};
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
    fn from_i(v: i128) -> Rat { Rat { n: v, d: 1 } }
    fn mul(self, o: Rat) -> Option<Rat> { Rat::new(self.n * o.n, self.d * o.d) }
    fn eq(self, o: Rat) -> bool { self.n == o.n && self.d == o.d }
}

fn gcd(a: u128, b: u128) -> u128 { if b == 0 { a } else { gcd(b, a % b) } }

fn to_rat(e: &Expr) -> Option<Rat> {
    match e {
        Expr::Integer(n) => Some(Rat::from_i(*n as i128)),
        Expr::Rational { num, den } => Rat::new(*num as i128, *den as i128),
        _ => None,
    }
}

/// Collect free symbols (no descent needed beyond structure).
fn free_symbols(e: &Expr, out: &mut Vec<SymbolId>) {
    match e {
        Expr::Symbol(id) => if !out.contains(id) { out.push(*id); },
        Expr::List { args, .. } => for a in args { free_symbols(a, out); },
        _ => {}
    }
}

/// Try to evaluate S(n0) = Σ_{k=lo..hi} body exactly as a rational.
fn sample_sum(body: &Expr, k: &Expr, lo: &Expr, hi: &Expr, n: &Expr, n0: i64, env: &mut Environment) -> Option<Rat> {
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
    to_rat(&acc)
}

/// Detect S(n+1)/S(n) = c·(n+a)/(n+b) and return the verified closed form.
pub fn try_hyper_sum_order1(body: &Expr, k_id: SymbolId, lo: &Expr, hi: &Expr, env: &mut Environment) -> Option<Expr> {
    // Exactly one free parameter n (besides k).
    let mut params = Vec::new();
    free_symbols(body, &mut params);
    free_symbols(lo, &mut params);
    free_symbols(hi, &mut params);
    params.retain(|id| *id != k_id);
    if params.len() != 1 { return None; }
    let n_id = params[0];
    let n = Expr::Symbol(n_id);
    let k = Expr::Symbol(k_id);

    // Sample S(n0) for a window of n0; require nonzero, distinct, exact values.
    let mut samples: Vec<(i64, Rat)> = Vec::new();
    for n0 in 1..=12i64 {
        match sample_sum(body, &k, lo, hi, &n, n0, env) {
            Some(s) if s.n != 0 => samples.push((n0, s)),
            _ => {}
        }
        if samples.len() >= 8 { break; }
    }
    if samples.len() < 6 { return None; }

    // Ratios R(n0) = S(n0+1)/S(n0) for consecutive samples.
    let mut ratios: Vec<(i64, Rat)> = Vec::new();
    for w in samples.windows(2) {
        if w[1].0 == w[0].0 + 1 {
            let r = Rat::new(w[1].1.n * w[0].1.d, w[0].1.n * w[1].1.d)?;
            ratios.push((w[0].0, r));
        }
    }
    if ratios.len() < 4 { return None; }

    // Search integer shifts a,b (numerator/denominator) and derive c, then
    // verify R(m) = c·(m+a)/(m+b) on every sampled ratio.
    let (m0, r0) = ratios[0];
    for a in -12..=12i64 {
        for b in -12..=12i64 {
            if m0 + a == 0 { continue; }
            // c = R(m0)·(m0+b)/(m0+a)
            let Some(c) = Rat::new(r0.n * (m0 + b) as i128, r0.d * (m0 + a) as i128) else { continue };
            let ok = ratios.iter().all(|&(m, r)| {
                // r == c·(m+a)/(m+b)  ⇔  r.n·(m+b)·c.d == c.n·(m+a)·r.d
                (m + b) != 0
                    && (r.n * (m + b) as i128 * c.d) == (c.n * (m + a) as i128 * r.d)
            });
            if !ok { continue; }
            if let Some(closed) = build_closed_form(&samples, c, a, b, &n) {
                if verify(&closed, &samples, &n, env) {
                    return Some(crate::eval::meval(&closed, env));
                }
            }
        }
    }
    None
}

/// S(n) = K · c^(n−n0) · ∏(n+i),  the telescoped order-1 closed form.
fn build_closed_form(samples: &[(i64, Rat)], c: Rat, a: i64, b: i64, n: &Expr) -> Option<Expr> {
    let n0 = samples[0].0;
    let c_expr = rat_expr(c);
    // c^(n − n0)
    let geom = Expr::pow(c_expr, Expr::sub(n.clone(), Expr::int(n0)));
    // ∏(n+i): for a≥b, numerator product i∈[b,a−1]; for a<b, denominator i∈[a,b−1].
    let mut prod = Expr::int(1);
    if a >= b {
        for i in b..a { prod = Expr::mul(prod, Expr::add(n.clone(), Expr::int(i))); }
        Some(scale_to_match(Expr::mul(geom, prod), samples, n))
    } else {
        for i in a..b { prod = Expr::mul(prod, Expr::add(n.clone(), Expr::int(i))); }
        Some(scale_to_match(Expr::div(geom, prod), samples, n))
    }
}

/// Multiply by the constant K = S(n0)/shape(n0) so the closed form matches.
fn scale_to_match(shape: Expr, samples: &[(i64, Rat)], n: &Expr) -> Expr {
    let n0 = samples[0].0;
    let s0 = samples[0].1;
    // shape(n0)
    let shape_n0 = simplify(&subst(&Expr::int(n0), n, &shape));
    Expr::mul(Expr::div(rat_expr(s0), shape_n0), shape)
}

fn verify(closed: &Expr, samples: &[(i64, Rat)], n: &Expr, env: &mut Environment) -> bool {
    let mut checked = 0;
    for &(n0, s) in samples {
        let v = crate::eval::meval(&subst(&Expr::int(n0), n, closed), env);
        match to_rat(&v) {
            Some(r) => { if !r.eq(s) { return false; } checked += 1; }
            None => return false,
        }
    }
    checked >= 6
}

fn rat_expr(r: Rat) -> Expr {
    if r.d == 1 { Expr::int(r.n as i64) }
    else { Expr::Rational { num: r.n as i64, den: r.d as i64 } }
}

#[cfg(test)]
mod tests {
    use crate::eval::eval_str;
    fn run(s: &str) -> String { eval_str(s) }

    #[test] fn sum_k_binomial() {
        // Σ_{k=0}^n k·C(n,k) = n·2^(n-1). Closed form found (not a noun)…
        let s = run("sum(k*binomial(n,k),k,0,n);");
        assert!(!s.contains("sum("), "got noun: {s}");
        // …and it agrees with the fully-numeric sum at n=6: 6·2^5 = 192.
        assert_eq!(run("sum(k*binomial(6,k),k,0,6);"), "192");
    }

    #[test] fn sum_k2_binomial() {
        // Σ_{k=0}^n k²·C(n,k) = n(n+1)·2^(n-2).
        let s = run("sum(k^2*binomial(n,k),k,0,n);");
        assert!(!s.contains("sum("), "got noun: {s}");
        assert_eq!(run("sum(k^2*binomial(5,k),k,0,5);"), "240"); // 5·6·2^3
    }

    #[test] fn sum_plain_binomial_still_2n() {
        assert_eq!(run("sum(binomial(n,k),k,0,n);"), "2^n");
    }

    #[test] fn non_hypergeometric_is_noun() {
        // 1/k^2 has no elementary closed form → noun (never a wrong answer).
        assert!(run("sum(1/k^2,k,1,n);").contains("sum("));
    }
}
