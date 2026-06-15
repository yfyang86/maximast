// V9.0 / V4: ∫ R(x)·exp(c·x) dx with rational R — Risch DE B'+cB=R solved for a
// rational B via the ansatz B=M/Q. Verified by differentiate-back.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn rational_b_elementary() {
    assert_eq!(run("integrate(x*exp(x)/(x+1)^2, x);"), "exp(x)/(1+x)");
    assert_eq!(run("integrate((x-1)*exp(x)/x^2, x);"), "exp(x)/x");
}

#[test]
fn polynomial_times_exp_unchanged() {
    assert_eq!(run("integrate(x*exp(x), x);"), "(-1+x)*exp(x)");
    let r = run("integrate((x^2+1)*exp(x), x);");
    assert!(r.contains("exp(x)") && !r.contains("integrate"), "got: {}", r);
}

#[test]
fn nonelementary_stays_noun() {
    // ∫exp(x)/(x+1)^2 is nonelementary (relates to Ei) — must stay a noun form.
    assert!(run("integrate(exp(x)/(x+1)^2, x);").contains("integrate"));
}
