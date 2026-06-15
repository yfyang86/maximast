// V8.0 / S4: robust power-series engine — Laurent series via numerator/
// denominator series division, plus reduced Taylor coefficients.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn laurent_csc_and_ratio_poles() {
    // 1/(exp(x)-1) = 1/x - 1/2 + x/12 - x^3/720 + ...  (Laurent, pole order 1)
    let r = run("taylor(1/(exp(x)-1), x, 0, 4);");
    assert!(!r.contains("und") && !r.contains("inf"), "got: {}", r);
    assert!(r.contains("1/x") && r.contains("(1/12)*x"), "got: {}", r);
    // 1/sin(x) = 1/x + x/6 + 7x^3/360 + ...
    let r = run("taylor(1/sin(x), x, 0, 4);");
    assert!(r.contains("1/x") && r.contains("(1/6)*x"), "got: {}", r);
    // cos(x)/x = 1/x - x/2 + x^3/24 + ...
    let r = run("taylor(cos(x)/x, x, 0, 4);");
    assert!(r.contains("1/x"), "got: {}", r);
}

#[test]
fn taylor_reduced_coefficients() {
    // tan(x) = x + x^3/3 + 2x^5/15 + 17x^7/315
    let r = run("taylor(tan(x), x, 0, 7);");
    assert!(r.contains("(1/3)*x^3") && r.contains("(2/15)*x^5") && r.contains("(17/315)*x^7"), "got: {}", r);
    // sqrt(1+x) = 1 + x/2 - x^2/8 + x^3/16 - 5x^4/128
    let r = run("taylor(sqrt(1+x), x, 0, 4);");
    assert!(r.contains("(1/2)*x") && r.contains("(1/8)*x^2") && r.contains("(5/128)*x^4"), "got: {}", r);
}

#[test]
fn series_backed_limits_unchanged() {
    assert_eq!(run("limit((tan(x)-x)/x^3, x, 0);"), "1/3");
    assert_eq!(run("limit(sin(x)/x, x, 0);"), "1");
    assert_eq!(run("limit((1-cos(x))/x^2, x, 0);"), "1/2");
}
