//! Special functions as a Maxima plugin (numeric, f64).
//!
//! Functions: gamma, log_gamma, beta, erf, erfc, bessel_j, bessel_i.
//! These return a Float for a numeric (Float) argument, with a few exact
//! cases: `gamma(n)` = (n-1)!, `gamma(1/2)` = sqrt(%pi), `beta(m,n)` for
//! positive integers, `erf(0)` = 0, `erfc(0)` = 1. A symbolic (or otherwise
//! unsupported) argument yields the noun form.
//!
//! Every numeric routine has a fixed iteration cap and is checked against
//! reference values in the test suite (see crates/eval/tests/specfun_test.rs).
//!
//! Deferred: bessel_y and bessel_k (second-kind). For integer order they
//! require the digamma function and logarithmic terms; a focused follow-up.

use maxima_plugin::{maxima_plugin, Expr, Environment, guard};
use num::{BigInt, BigRational, One, ToPrimitive};
use std::f64::consts::PI;

// ---- numeric helpers -----------------------------------------------------

fn as_f64(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        Expr::BigInt(b) => b.to_f64(),
        _ => None,
    }
}
fn is_float(e: &Expr) -> bool { matches!(e, Expr::Float(_)) }

fn fact_bigint(n: u64) -> BigInt {
    (1..=n).fold(BigInt::one(), |acc, k| acc * BigInt::from(k))
}
fn bigint_to_expr(b: &BigInt) -> Expr {
    match b.to_i64() {
        Some(i) => Expr::int(i),
        None => Expr::BigInt(Box::new(b.clone())),
    }
}
fn rational_to_expr(num: BigInt, den: BigInt) -> Expr {
    let q = BigRational::new(num, den);
    if q.denom().is_one() {
        bigint_to_expr(q.numer())
    } else {
        Expr::div(bigint_to_expr(q.numer()), bigint_to_expr(q.denom()))
    }
}

// ---- core numeric kernels ------------------------------------------------

// Lanczos approximation (g=7, n=9), good to ~1e-15.
const LANCZOS_G: f64 = 7.0;
const LANCZOS: [f64; 9] = [
    0.999_999_999_999_809_93,
    676.520_368_121_885_1,
    -1259.139_216_722_402_8,
    771.323_428_777_653_13,
    -176.615_029_162_140_59,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

fn lanczos_gamma(x: f64) -> f64 {
    if x < 0.5 {
        PI / ((PI * x).sin() * lanczos_gamma(1.0 - x))
    } else {
        let x = x - 1.0;
        let mut a = LANCZOS[0];
        let t = x + LANCZOS_G + 0.5;
        for (i, &c) in LANCZOS.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
    }
}

fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        (PI / (PI * x).sin()).abs().ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = LANCZOS[0];
        let t = x + LANCZOS_G + 0.5;
        for (i, &c) in LANCZOS.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

fn erf_series(x: f64) -> f64 {
    // (2/sqrt(pi)) * sum_n (-1)^n x^(2n+1) / (n! (2n+1))
    let x2 = x * x;
    let mut term = x; // (-1)^n x^(2n+1)/n!  at n=0
    let mut sum = x; // term/(2n+1) at n=0
    for n in 1..200 {
        term *= -x2 / n as f64;
        let add = term / (2.0 * n as f64 + 1.0);
        sum += add;
        if add.abs() < 1e-18 {
            break;
        }
    }
    2.0 / PI.sqrt() * sum
}

fn erfc_cf(x: f64) -> f64 {
    // erfc(x) = exp(-x^2)/sqrt(pi) * 1/(x + a1/(x + a2/(x + ...))), a_k = k/2
    let mut t = 0.0;
    for k in (1..=200).rev() {
        t = (k as f64 / 2.0) / (x + t);
    }
    (-x * x).exp() / PI.sqrt() / (x + t)
}

fn erf(x: f64) -> f64 {
    if x < 0.0 { -erf(-x) }
    else if x < 2.0 { erf_series(x) }
    else { 1.0 - erfc_cf(x) }
}
fn erfc(x: f64) -> f64 {
    if x < 0.0 { 2.0 - erfc(-x) }
    else if x < 2.0 { 1.0 - erf_series(x) }
    else { erfc_cf(x) }
}

// Bessel via the ascending series (requires nu >= 0, x >= 0).
fn bessel_series(nu: f64, x: f64, alternating: bool) -> f64 {
    let h = x / 2.0;
    let h2 = h * h;
    let mut term = h.powf(nu) / lanczos_gamma(nu + 1.0);
    let mut sum = term;
    for m in 1..500 {
        let factor = h2 / (m as f64 * (m as f64 + nu));
        term *= if alternating { -factor } else { factor };
        sum += term;
        if term.abs() < sum.abs().max(1.0) * 1e-18 {
            break;
        }
    }
    sum
}
fn bessel_j(nu: f64, x: f64) -> f64 { bessel_series(nu, x, true) }
fn bessel_i(nu: f64, x: f64) -> f64 { bessel_series(nu, x, false) }

// ---- function dispatch ---------------------------------------------------

fn float_or_noun(name: &str, args: &[Expr], v: f64) -> Expr {
    if v.is_finite() { Expr::Float(v) } else { Expr::call(name, args.to_vec()) }
}

fn gamma_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("gamma", args, || match &args[0] {
        Expr::Integer(n) if *n >= 1 => bigint_to_expr(&fact_bigint((*n - 1) as u64)),
        Expr::Rational { num: 1, den: 2 } => Expr::call("sqrt", vec![Expr::sym("%pi")]),
        Expr::Float(x) => float_or_noun("gamma", args, lanczos_gamma(*x)),
        _ => Expr::call("gamma", args.to_vec()),
    })
}

fn log_gamma_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("log_gamma", args, || match &args[0] {
        Expr::Float(x) if *x > 0.0 => float_or_noun("log_gamma", args, ln_gamma(*x)),
        _ => Expr::call("log_gamma", args.to_vec()),
    })
}

fn beta_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("beta", args, || {
        let (a, b) = (&args[0], &args[1]);
        if let (Expr::Integer(m), Expr::Integer(n)) = (a, b) {
            if *m >= 1 && *n >= 1 {
                let num = fact_bigint((*m - 1) as u64) * fact_bigint((*n - 1) as u64);
                let den = fact_bigint((*m + *n - 1) as u64);
                return rational_to_expr(num, den);
            }
        }
        if is_float(a) || is_float(b) {
            if let (Some(av), Some(bv)) = (as_f64(a), as_f64(b)) {
                let v = lanczos_gamma(av) * lanczos_gamma(bv) / lanczos_gamma(av + bv);
                return float_or_noun("beta", args, v);
            }
        }
        Expr::call("beta", args.to_vec())
    })
}

fn erf_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("erf", args, || match &args[0] {
        Expr::Integer(0) => Expr::int(0),
        Expr::Float(x) => float_or_noun("erf", args, erf(*x)),
        _ => Expr::call("erf", args.to_vec()),
    })
}

fn erfc_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("erfc", args, || match &args[0] {
        Expr::Integer(0) => Expr::int(1),
        Expr::Float(x) => float_or_noun("erfc", args, erfc(*x)),
        _ => Expr::call("erfc", args.to_vec()),
    })
}

fn bessel_j_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("bessel_j", args, || {
        if let (Some(nu), Some(x)) = (as_f64(&args[0]), as_f64(&args[1])) {
            if is_float(&args[1]) && nu >= 0.0 && x >= 0.0 {
                return float_or_noun("bessel_j", args, bessel_j(nu, x));
            }
        }
        Expr::call("bessel_j", args.to_vec())
    })
}

fn bessel_i_fn(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("bessel_i", args, || {
        if let (Some(nu), Some(x)) = (as_f64(&args[0]), as_f64(&args[1])) {
            if is_float(&args[1]) && nu >= 0.0 && x >= 0.0 {
                return float_or_noun("bessel_i", args, bessel_i(nu, x));
            }
        }
        Expr::call("bessel_i", args.to_vec())
    })
}

maxima_plugin!(register = |env| {
    env.register_native("gamma", gamma_fn, 1, Some(1));
    env.register_native("log_gamma", log_gamma_fn, 1, Some(1));
    env.register_native("beta", beta_fn, 2, Some(2));
    env.register_native("erf", erf_fn, 1, Some(1));
    env.register_native("erfc", erfc_fn, 1, Some(1));
    env.register_native("bessel_j", bessel_j_fn, 2, Some(2));
    env.register_native("bessel_i", bessel_i_fn, 2, Some(2));
});
