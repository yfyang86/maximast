// V8.0 / S6: algebraic (radical) integration. Polynomial numerators over a
// quadratic radical via the R·√Q + λ∫1/√Q reduction; rational-power
// substitutions; and correct noun form for genuinely nonelementary (elliptic)
// integrands. All closed forms pass the numeric differentiate-back gate.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn polynomial_numerator_over_sqrt_quadratic() {
    // ∫(x²+1)/√(x²+x) = (x/2-3/4)√(x²+x) + (11/8)log(2x+1+2√(x²+x))
    let r = run("integrate((x^2+1)/sqrt(x^2+x), x);");
    assert!(!r.contains("integrate"), "got: {}", r);
    assert!(r.contains("sqrt(x+x^2)") && r.contains("log"), "got: {}", r);
    // ∫x²/√(x²+1) = x√(x²+1)/2 - log(x+√(x²+1))/2
    let r = run("integrate(x^2/sqrt(x^2+1), x);");
    assert!(!r.contains("integrate") && r.contains("log"), "got: {}", r);
}

#[test]
fn rational_power_substitutions() {
    assert_eq!(run("integrate(x/sqrt(x^4+1), x);"), "asinh(x^2)/2");
    assert_eq!(run("integrate(x^2/sqrt(1-x^6), x);"), "asin(x^3)/3");
}

#[test]
fn elliptic_stays_noun() {
    // Genuinely nonelementary — Trager's decision is "no elementary form".
    assert!(run("integrate(1/sqrt(x^3+1), x);").contains("integrate"));
}
