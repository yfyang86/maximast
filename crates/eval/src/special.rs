//! Named nonelementary special functions (V8.0 / S7).
//!
//! Built-in so the integration tower (S2/S5) always has a vocabulary of
//! named antiderivatives to return, independent of the optional `specfun`
//! plugin. Maxima-compatible names:
//!   erf, erfc, erfi,
//!   expintegral_ei, expintegral_li, expintegral_si, expintegral_ci,
//!   fresnel_s, fresnel_c.
//!
//! `diff` rules live in `eval::diff_once`; this module supplies the exact
//! special values (at 0) and `f64` numeric evaluation. Every kernel has a
//! fixed iteration cap and is checked against reference values in
//! `crates/eval/tests/special_test.rs` (mandatory numeric verification).

use maxima_core::Expr;
use std::f64::consts::PI;

/// Names handled by this module (single-argument).
pub fn is_special(name: &str) -> bool {
    matches!(
        name,
        "erf" | "erfc" | "erfi"
            | "expintegral_ei" | "expintegral_li"
            | "expintegral_si" | "expintegral_ci"
            | "fresnel_s" | "fresnel_c"
    )
}

/// Evaluate a special function at a single argument.
/// Returns `Some` for exact special values and floating-point arguments;
/// `None` (→ noun form) for symbolic arguments or singular points.
pub fn eval_special(name: &str, arg: &Expr) -> Option<Expr> {
    if !is_special(name) {
        return None;
    }

    // Exact special values at 0 (keep singular cases as noun forms).
    if let Expr::Integer(0) = arg {
        return match name {
            "erf" | "erfi" | "expintegral_si" | "fresnel_s" | "fresnel_c" => Some(Expr::int(0)),
            "erfc" => Some(Expr::int(1)),
            _ => None, // ei/li/ci are singular at 0
        };
    }

    // Numeric evaluation only for explicit floats (matches sin/cos behaviour:
    // erf(1) stays symbolic; float(erf(1)) evaluates).
    if let Expr::Float(x) = arg {
        let v = numeric(name, *x)?;
        if v.is_finite() {
            return Some(Expr::Float(v));
        }
    }
    None
}

fn numeric(name: &str, x: f64) -> Option<f64> {
    let v = match name {
        "erf" => erf(x),
        "erfc" => erfc(x),
        "erfi" => erfi(x),
        "expintegral_ei" => ei(x),
        "expintegral_li" => {
            if x <= 0.0 || x == 1.0 { return None; }
            ei(x.ln())
        }
        "expintegral_si" => si(x),
        "expintegral_ci" => {
            if x <= 0.0 { return None; }
            ci(x)
        }
        "fresnel_s" => fresnel_s(x),
        "fresnel_c" => fresnel_c(x),
        _ => return None,
    };
    Some(v)
}

// ---- numeric kernels (capped series / continued fraction) ----------------

// erf via the same series / continued-fraction split as the specfun plugin.
fn erf_series(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for n in 1..200 {
        term *= -x2 / n as f64;
        let add = term / (2.0 * n as f64 + 1.0);
        sum += add;
        if add.abs() < 1e-18 { break; }
    }
    2.0 / PI.sqrt() * sum
}

fn erfc_cf(x: f64) -> f64 {
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

// erfi(x) = (2/sqrt(pi)) * integral_0^x exp(t^2) dt  (no sign alternation).
fn erfi(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for n in 1..200 {
        term *= x2 / n as f64;
        let add = term / (2.0 * n as f64 + 1.0);
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 { break; }
    }
    2.0 / PI.sqrt() * sum
}

const EULER_MASCHERONI: f64 = 0.577_215_664_901_532_9;

// Ei(x) = gamma + ln|x| + sum_{k>=1} x^k / (k * k!).
fn ei(x: f64) -> f64 {
    let mut term = 1.0; // x^k / k!  at k=1 build-up
    let mut sum = 0.0;
    for k in 1..200 {
        term *= x / k as f64;
        let add = term / k as f64;
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 && k > 2 { break; }
    }
    EULER_MASCHERONI + x.abs().ln() + sum
}

// Si(x) = sum_{n>=0} (-1)^n x^(2n+1) / ((2n+1)(2n+1)!).
fn si(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x; // x^(2n+1)/(2n+1)!  at n=0
    let mut sum = x;
    for n in 1..200 {
        term *= -x2 / ((2 * n) as f64 * (2 * n + 1) as f64);
        let add = term / (2 * n + 1) as f64;
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 { break; }
    }
    sum
}

// Ci(x) = gamma + ln(x) + sum_{n>=1} (-1)^n x^(2n) / ((2n)(2n)!).
fn ci(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = 1.0; // x^(2n)/(2n)!  at n=0
    let mut sum = 0.0;
    for n in 1..200 {
        term *= -x2 / ((2 * n - 1) as f64 * (2 * n) as f64);
        let add = term / (2 * n) as f64;
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 && n > 1 { break; }
    }
    EULER_MASCHERONI + x.ln() + sum
}

// S(x) = sum_{n>=0} (-1)^n (pi/2)^(2n+1) x^(4n+3) / ((2n+1)! (4n+3)).
fn fresnel_s(x: f64) -> f64 {
    let h = PI / 2.0;
    let mut sum = 0.0;
    let mut sign = 1.0;
    let mut hp = h; // (pi/2)^(2n+1)
    let mut fact = 1.0; // (2n+1)!
    for n in 0..120 {
        if n > 0 {
            hp *= h * h;
            fact *= (2 * n) as f64 * (2 * n + 1) as f64;
            sign = -sign;
        }
        let exp = (4 * n + 3) as i32;
        let add = sign * hp * x.powi(exp) / (fact * (4 * n + 3) as f64);
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 && n > 1 { break; }
    }
    sum
}

// C(x) = sum_{n>=0} (-1)^n (pi/2)^(2n) x^(4n+1) / ((2n)! (4n+1)).
fn fresnel_c(x: f64) -> f64 {
    let h = PI / 2.0;
    let mut sum = 0.0;
    let mut sign = 1.0;
    let mut hp = 1.0; // (pi/2)^(2n)
    let mut fact = 1.0; // (2n)!
    for n in 0..120 {
        if n > 0 {
            hp *= h * h;
            fact *= (2 * n - 1) as f64 * (2 * n) as f64;
            sign = -sign;
        }
        let exp = (4 * n + 1) as i32;
        let add = sign * hp * x.powi(exp) / (fact * (4 * n + 1) as f64);
        sum += add;
        if add.abs() < sum.abs().max(1.0) * 1e-18 && n > 1 { break; }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) { assert!((a - b).abs() < 1e-9, "{} vs {}", a, b); }

    #[test]
    fn reference_values() {
        close(erf(1.0), 0.842_700_792_949_714_9);
        close(erfc(0.5), 0.479_500_122_186_953_5);
        close(erfi(1.0), 1.650_425_758_797_542_8);
        close(ei(1.0), 1.895_117_816_355_936_8);
        close(si(1.0), 0.946_083_070_367_183_0);
        close(ci(1.0), 0.337_403_922_900_968_1);
        close(fresnel_s(1.0), 0.438_259_147_390_354_8);
        close(fresnel_c(1.0), 0.779_893_400_376_822_8);
        close(ei(2.0_f64.ln()), 1.045_163_780_117_492_7); // li(2)
    }
}
