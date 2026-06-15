// V8.0 / S2: Risch DE solver improvements + named nonelementary results.
// Covers the substitution correctness fix (Gaussian-type elementary integrals)
// and the named special-function antiderivatives.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn gaussian_elementary_now_correct() {
    // Regression: these previously returned a wrong cube form (2*exp(x^2)^3/3).
    assert_eq!(run("integrate(x*exp(x^2), x);"), "exp(x^2)/2");
    assert_eq!(run("integrate((2*x+1)*exp(x^2+x), x);"), "exp(x+x^2)");
}

#[test]
fn gaussian_named_erf_erfi() {
    let r = run("integrate(exp(x^2), x);");
    assert!(r.contains("erfi(x)") && !r.contains("integrate"), "got: {}", r);
    let r = run("integrate(exp(-x^2), x);");
    assert!(r.contains("erf(x)") && !r.contains("erfi") && !r.contains("integrate"), "got: {}", r);
}

#[test]
fn logarithmic_and_exponential_integrals() {
    assert_eq!(run("integrate(1/log(x), x);"), "expintegral_li(x)");
    assert_eq!(run("integrate(exp(x)/x, x);"), "expintegral_ei(x)");
    assert_eq!(run("integrate(exp(2*x)/x, x);"), "expintegral_ei(2*x)");
}

#[test]
fn sine_cosine_integrals() {
    assert_eq!(run("integrate(sin(x)/x, x);"), "expintegral_si(x)");
    assert_eq!(run("integrate(cos(x)/x, x);"), "expintegral_ci(x)");
}

#[test]
fn elementary_paths_unchanged() {
    // Ordinary integrals must still resolve elementarily, not to special fns.
    assert_eq!(run("integrate(exp(x), x);"), "exp(x)");
    assert_eq!(run("integrate(1/x, x);"), "log(x)");
}
