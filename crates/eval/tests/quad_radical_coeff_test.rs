// V9.0 / V1: regression for the ∫1/√(ax²+bx+c) leading-coefficient bug.
// A table handler used to ignore a≠1 and return log(x+√(1+x²)) for 1/√(4x²+1).
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn non_monic_leading_coefficient() {
    // d/dx[asinh(2x)/2] = 1/√(4x²+1)  — the old bug returned log(x+√(1+x²)).
    assert_eq!(run("integrate(1/sqrt(4*x^2+1), x);"), "asinh(2*x)/2");
    // a=9, c=4: arg 3x/2
    assert_eq!(run("integrate(1/sqrt(9*x^2+4), x);"), "asinh((3/2)*x)/3");
}

#[test]
fn monic_cases_unchanged() {
    assert_eq!(run("integrate(1/sqrt(x^2+1), x);"), "asinh(x)");
    assert_eq!(run("integrate(1/sqrt(x^2-1), x);"), "acosh(x)");
    assert_eq!(run("integrate(1/sqrt(1-x^2), x);"), "asin(x)");
}

#[test]
fn non_perfect_square_leading_coeff() {
    // a=2 (not a perfect square): verified log form, must not be wrong.
    let r = run("integrate(1/sqrt(2*x^2-3), x);");
    assert!(!r.contains("integrate") && r.contains("log") && r.contains("sqrt(2)"), "got: {}", r);
}
