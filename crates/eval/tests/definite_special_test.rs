// V8.0 / S5: special-function limit values at ±∞ that let definite integrals
// over (a, ∞) using named antiderivatives collapse to clean closed forms.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn special_function_limits() {
    assert_eq!(run("erf(inf);"), "1");
    assert_eq!(run("erf(minf);"), "-1");
    assert_eq!(run("erfc(inf);"), "0");
    assert_eq!(run("expintegral_si(inf);"), "%pi/2");
}

#[test]
fn definite_integrals_via_named_antiderivatives() {
    // ∫₀^∞ exp(-x²) = √π/2
    assert_eq!(run("integrate(exp(-x^2), x, 0, inf);"), "(1/2)*sqrt(%pi)");
    // Dirichlet integral ∫₀^∞ sin(x)/x = π/2
    assert_eq!(run("integrate(sin(x)/x, x, 0, inf);"), "%pi/2");
    // ∫₋∞^∞ exp(-x²) = √π
    assert_eq!(run("integrate(exp(-x^2), x, minf, inf);"), "sqrt(%pi)");
}

#[test]
fn parametrized_families_unchanged() {
    assert_eq!(run("integrate(exp(-x^2)*x^2, x, 0, inf);"), "sqrt(%pi)/4");
    assert_eq!(run("integrate(x^3*exp(-x), x, 0, inf);"), "6");
}
