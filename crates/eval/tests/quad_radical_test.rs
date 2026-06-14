// V8.0 / S3: quadratic-radical integrals ∫ R(x, √(ax²+bx+c)) dx via
// completing-the-square (Families A/B/C). Every engine result is verified by
// differentiating back, so these assertions check the engine fires and yields
// the expected elementary form.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn sqrt_quadratic_family_c() {
    // ∫ √(x²-1) dx  → rational·√ + log
    let r = run("integrate(sqrt(x^2-1), x);");
    assert!(!r.contains("integrate"), "got: {}", r);
    assert!(r.contains("log") && r.contains("sqrt"), "got: {}", r);
    // ∫ √(4-x²) dx  → rational·√ + asin
    let r = run("integrate(sqrt(4-x^2), x);");
    assert!(!r.contains("integrate") && r.contains("asin"), "got: {}", r);
}

#[test]
fn inv_sqrt_quadratic_family_a() {
    assert_eq!(run("integrate(1/sqrt(x^2+1), x);"), "asinh(x)");
    assert_eq!(run("integrate(1/sqrt(1-x^2), x);"), "asin(x)");
}

#[test]
fn linear_over_sqrt_family_b() {
    // ∫ (2x+3)/√(x²+1) dx = 2√(x²+1) + 3·asinh(x)
    let r = run("integrate((2*x+3)/sqrt(x^2+1), x);");
    assert!(!r.contains("integrate"), "got: {}", r);
    assert!(r.contains("asinh") && r.contains("sqrt(1+x^2)"), "got: {}", r);
    // ∫ x/√(2x-x²) dx (a<0) → -√(2x-x²) + asin(...)
    let r = run("integrate(x/sqrt(2*x-x^2), x);");
    assert!(!r.contains("integrate") && r.contains("asin"), "got: {}", r);
}
