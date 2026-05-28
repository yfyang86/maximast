use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// S4: Polynomial analysis
#[test] fn resultant_basic() { assert_eq!(run("resultant(x^2+1, x+1, x);"), "2"); }
#[test] fn resultant_linear() {
    let r = run("resultant(x-2, x-3, x);");
    assert!(r == "1" || r == "-1", "got: {}", r);
}

#[test] fn discriminant_quadratic() {
    assert_eq!(run("discriminant(x^2-5*x+6, x);"), "1");
}
#[test] fn discriminant_zero() {
    assert_eq!(run("discriminant(x^2-2*x+1, x);"), "0");
}

#[test] fn content_integer() { assert_eq!(run("content(6*x^2+4*x+2, x);"), "2"); }
#[test] fn content_one() { assert_eq!(run("content(x^2+x+1, x);"), "1"); }

#[test] fn primpart_basic() {
    let r = run("primpart(6*x^2+4*x+2, x);");
    // Should be 3x^2+2x+1 — check it doesn't have factor 2
    assert!(!r.contains("6") && !r.contains("4"), "should remove content, got: {}", r);
}

// S5: Log/trig simplification
#[test] fn logcontract_sum() {
    assert_eq!(run("logcontract(log(x)+log(y));"), "log(x*y)");
}
#[test] fn logcontract_numeric() {
    assert_eq!(run("logcontract(log(2)+log(3));"), "log(6)");
}
#[test] fn logcontract_coeff() {
    assert_eq!(run("logcontract(2*log(x));"), "log(x^2)");
}

#[test] fn logexpand_product() {
    let r = run("logexpand(log(x*y));");
    assert!(r.contains("log(x)") && r.contains("log(y)"), "got: {}", r);
}
#[test] fn logexpand_power() {
    assert_eq!(run("logexpand(log(x^3));"), "3*log(x)");
}
#[test] fn logexpand_numeric() {
    let r = run("logexpand(log(12));");
    // log(12) = log(4*3) or log(2^2*3) — may or may not expand
    assert!(r.contains("log"), "got: {}", r);
}

#[test] fn logcontract_logexpand_roundtrip() {
    let expanded = run("logexpand(log(x*y));");
    let contracted = run(&format!("logcontract({});", expanded));
    assert!(contracted.contains("log") && contracted.contains("x") && contracted.contains("y"),
        "got: {}", contracted);
}
