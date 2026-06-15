// V9.0 / V3: Puiseux (fractional-exponent) series about 0 for f = x^q · g(x).
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn puiseux_basic() {
    assert_eq!(run("taylor(sqrt(x), x, 0, 3);"), "x^(1/2)");
    // sqrt(x)*cos(x) = x^(1/2) - x^(5/2)/2 + ...
    let r = run("taylor(sqrt(x)*cos(x), x, 0, 4);");
    assert!(r.contains("x^(1/2)") && r.contains("x^(5/2)"), "got: {}", r);
    // cube-root: x^(1/3)*sin(x) = x^(4/3) - ...  (previously a spurious 0)
    assert_eq!(run("taylor(x^(1/3)*sin(x), x, 0, 3);"), "x^(4/3)");
}

#[test]
fn puiseux_exp() {
    // sqrt(x)*exp(x) = x^(1/2) + x^(3/2) + x^(5/2)/2 + ...
    let r = run("taylor(sqrt(x)*exp(x), x, 0, 3);");
    assert!(r.contains("x^(1/2)") && r.contains("x^(3/2)") && r.contains("x^(5/2)"), "got: {}", r);
}

#[test]
fn ordinary_and_laurent_unchanged() {
    assert_eq!(run("taylor(sin(x), x, 0, 5);"), "x-(1/6)*x^3+(1/120)*x^5");
    let r = run("taylor(1/(exp(x)-1), x, 0, 4);");
    assert!(r.contains("1/x") && r.contains("(1/12)*x"), "got: {}", r);
}
