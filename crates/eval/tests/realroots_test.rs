// Bundle 2 / 1b: exact real-root isolation. realroots factors over Q (linear
// factors → exact rational roots) and isolates each irreducible factor's real
// roots by Sturm bisection in exact rational arithmetic, returning a rational
// within eps. Output is Maxima-style `[x = r, ...]` of exact rationals — no
// f64. Each verified by checking the rational lands within eps of the true root.
use maxima_eval::{eval_str_with_env, Environment};
use num::BigRational;
use std::str::FromStr;

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

/// Pull the rational on the RHS of each `x = r` and return them sorted as f64.
fn roots_f64(s: &str) -> Vec<f64> {
    s.trim_start_matches('[').trim_end_matches(']')
        .split(',')
        .filter(|t| !t.is_empty())
        .map(|t| {
            let rhs = t.split('=').nth(1).unwrap();
            match BigRational::from_str(rhs) {
                Ok(r) => ratio_f64(&r),
                Err(_) => rhs.parse::<f64>().unwrap(),
            }
        })
        .collect()
}

fn ratio_f64(r: &BigRational) -> f64 {
    use num::ToPrimitive;
    r.numer().to_f64().unwrap() / r.denom().to_f64().unwrap()
}

#[test]
fn exact_rational_roots() {
    // integer roots returned exactly, no float
    assert_eq!(run("realroots(x^2-1);"), "[x=-1,x=1]");
    assert_eq!(run("realroots(x^3-x);"), "[x=-1,x=0,x=1]");
    assert_eq!(run("realroots(2*x-3);"), "[x=3/2]");
}

#[test]
fn no_real_roots() {
    assert_eq!(run("realroots(x^2+1);"), "[]");
}

#[test]
fn multiplicity_listed_once() {
    // (x-1)^2 (x+3): the double root appears once (Maxima style)
    assert_eq!(run("realroots((x-1)^2*(x+3));"), "[x=-3,x=1]");
}

#[test]
fn irrational_within_eps() {
    // ±sqrt(2) as exact rationals within the default 1e-10
    let rs = roots_f64(&run("realroots(x^2-2);"));
    assert_eq!(rs.len(), 2);
    assert!((rs[0] + 2f64.sqrt()).abs() < 1e-10, "got {}", rs[0]);
    assert!((rs[1] - 2f64.sqrt()).abs() < 1e-10, "got {}", rs[1]);
    // output must be exact rationals, never f64 (no decimal point)
    assert!(!run("realroots(x^2-2);").contains('.'), "float leaked");
}

#[test]
fn cube_root_single_real() {
    // x^3-2 has one real root 2^(1/3); the two complex roots are excluded
    let rs = roots_f64(&run("realroots(x^3-2);"));
    assert_eq!(rs.len(), 1);
    assert!((rs[0] - 2f64.powf(1.0 / 3.0)).abs() < 1e-10, "got {}", rs[0]);
}

#[test]
fn custom_eps_coarser() {
    // a coarse tolerance still brackets sqrt(2) within 1e-3
    let rs = roots_f64(&run("realroots(x^2-2, 1/1000);"));
    assert_eq!(rs.len(), 2);
    assert!((rs[1] - 2f64.sqrt()).abs() < 1e-3, "got {}", rs[1]);
}
