//! Special-function numeric evaluation + key exact identities (V13 3c):
//! zeta, lambert_w, polylog. f64 precision (arbitrary precision would follow a
//! real bigfloat backend); exact closed forms are returned where known.

use maxima_core::Expr;
use crate::helpers::to_f64;

/// Riemann zeta for real s ≠ 1 via Euler–Maclaurin.
fn zeta_f64(s: f64) -> f64 {
    if s == 1.0 { return f64::INFINITY; }
    let n = 16usize;
    let nf = n as f64;
    let mut sum: f64 = (1..n).map(|k| (k as f64).powf(-s)).sum();
    sum += nf.powf(1.0 - s) / (s - 1.0) + 0.5 * nf.powf(-s);
    // Bernoulli tail: Σ B_{2k}/(2k)! · (s)_{2k-1} · n^{-s-2k+1}
    sum += (1.0 / 6.0) / 2.0 * s * nf.powf(-s - 1.0);
    sum += (-1.0 / 30.0) / 24.0 * s * (s + 1.0) * (s + 2.0) * nf.powf(-s - 3.0);
    sum += (1.0 / 42.0) / 720.0 * s * (s + 1.0) * (s + 2.0) * (s + 3.0) * (s + 4.0) * nf.powf(-s - 5.0);
    sum
}

/// Principal branch of the Lambert W function (W·e^W = x, x ≥ −1/e) via Halley.
fn lambert_w_f64(x: f64) -> Option<f64> {
    if x < -1.0 / std::f64::consts::E - 1e-12 { return None; }
    if x == 0.0 { return Some(0.0); }
    let mut w = if x > 1.0 { x.ln() } else { x };
    for _ in 0..80 {
        let ew = w.exp();
        let f = w * ew - x;
        let w1 = w + 1.0;
        let next = w - f / (ew * w1 - (w + 2.0) * f / (2.0 * w1));
        if (next - w).abs() <= 1e-15 * (1.0 + next.abs()) { return Some(next); }
        w = next;
    }
    Some(w)
}

/// Polylogarithm Li_s(z) for |z| < 1 (and z up to ~1) via the defining series.
fn polylog_f64(s: f64, z: f64) -> Option<f64> {
    if z.abs() >= 1.0 + 1e-12 { return None; } // series diverges outside the disk
    let mut sum = 0.0;
    let mut zn = z;
    for n in 1..100_000 {
        let term = zn / (n as f64).powf(s);
        sum += term;
        if term.abs() < 1e-16 * (1.0 + sum.abs()) { break; }
        zn *= z;
    }
    Some(sum)
}

/// Exact closed form of zeta at an integer, where one is standard.
fn zeta_exact(n: i64) -> Option<Expr> {
    match n {
        0 => Some(Expr::Rational { num: -1, den: 2 }),
        -1 => Some(Expr::Rational { num: -1, den: 12 }),
        // ζ(2k) = rational·π^{2k}
        2 => Some(Expr::div(Expr::pow(Expr::sym("%pi"), Expr::int(2)), Expr::int(6))),
        4 => Some(Expr::div(Expr::pow(Expr::sym("%pi"), Expr::int(4)), Expr::int(90))),
        6 => Some(Expr::div(Expr::pow(Expr::sym("%pi"), Expr::int(6)), Expr::int(945))),
        8 => Some(Expr::div(Expr::pow(Expr::sym("%pi"), Expr::int(8)), Expr::int(9450))),
        _ if n < 0 && n % 2 == 0 => Some(Expr::int(0)), // ζ(−2k)=0
        _ => None,
    }
}

/// Dispatch for the special-function builtins. None ⇒ fall through to a noun.
pub fn eval_specfun(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "zeta" if args.len() == 1 => {
            if let Expr::Integer(n) = &args[0] {
                if let Some(e) = zeta_exact(*n) { return Some(e); }
            }
            // numeric only for an explicit float argument
            if matches!(&args[0], Expr::Float(_)) {
                let s = to_f64(&args[0])?;
                return Some(Expr::Float(zeta_f64(s)));
            }
            None
        }
        "lambert_w" if args.len() == 1 => {
            if args[0] == Expr::int(0) { return Some(Expr::int(0)); }
            if matches!(&args[0], Expr::Float(_)) {
                return lambert_w_f64(to_f64(&args[0])?).map(Expr::Float);
            }
            None
        }
        "polylog" if args.len() == 2 => {
            // Li_2(1)=π²/6, Li_2(−1)=−π²/12 (exact).
            if let Expr::Integer(2) = &args[0] {
                if args[1] == Expr::int(1) {
                    return Some(Expr::div(Expr::pow(Expr::sym("%pi"), Expr::int(2)), Expr::int(6)));
                }
                if args[1] == Expr::int(-1) {
                    return Some(Expr::div(Expr::neg(Expr::pow(Expr::sym("%pi"), Expr::int(2))), Expr::int(12)));
                }
            }
            if matches!(&args[1], Expr::Float(_)) {
                let s = to_f64(&args[0])?;
                let z = to_f64(&args[1])?;
                return polylog_f64(s, z).map(Expr::Float);
            }
            None
        }
        _ => None,
    }
}
