// Regression: ∫sin(x)/cos(x) used to return 0 because the integrator's
// "∫ cos^n * sin = -cos^(n+1)/(n+1)" table entry matched with n=-1 and
// constructed -cos^0/0 = -1/0, which simplify silently collapsed to 0.
// Same shape for cos/sin, sinh/cosh, cosh/sinh. Now each n=-1 case returns
// the log antiderivative.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test] fn integrate_sin_over_cos() {
    assert_eq!(run("integrate(sin(x)/cos(x), x);"), "-log(cos(x))");
}
#[test] fn integrate_cos_over_sin() {
    assert_eq!(run("integrate(cos(x)/sin(x), x);"), "log(abs(sin(x)))");
}
#[test] fn integrate_sinh_over_cosh() {
    assert_eq!(run("integrate(sinh(x)/cosh(x), x);"), "log(cosh(x))");
}
#[test] fn integrate_cosh_over_sinh() {
    assert_eq!(run("integrate(cosh(x)/sinh(x), x);"), "log(abs(sinh(x)))");
}

// The classic table entries (n ≠ -1) must keep working.
#[test] fn integrate_sin_squared_times_cos_unchanged() {
    assert_eq!(run("integrate(sin(x)^2*cos(x), x);"), "sin(x)^3/3");
}
#[test] fn integrate_cos_cubed_times_sin_unchanged() {
    assert_eq!(norm(&run("integrate(cos(x)^3*sin(x), x);")), "-cos(x)^4/4");
}

// Same antiderivative as ∫tan(x), used to verify equivalence.
#[test] fn sin_over_cos_matches_tan() {
    assert_eq!(run("integrate(tan(x), x);"), "-log(cos(x))");
    assert_eq!(run("integrate(sin(x)/cos(x), x);"), "-log(cos(x))");
}

// The downstream win: S6's ODE solver can now solve y''+y=sec(x) via
// variation of parameters (was a known limitation in V6.0).
#[test] fn ode_with_sec_forcing_now_solves() {
    let r = run("ode2('diff(y,x,2)+y=1/cos(x), y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
    let n = norm(&r);
    assert!(n.contains("%k1*cos(x)") && n.contains("%k2*sin(x)"), "homogeneous part missing: {}", r);
    assert!(n.contains("x*sin(x)") && n.contains("cos(x)*log(cos(x))"), "particular part missing: {}", r);
}
