// V8.0 / S7: named nonelementary special functions as built-ins.
// Verifies exact special values, diff rules (so the S2/S5 verification loop
// can close), and float() evaluation against reference values.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

fn approx(s: &str, expected: f64) {
    let r = run(s);
    let v: f64 = r.parse().unwrap_or_else(|_| panic!("{} -> {} (not numeric)", s, r));
    assert!((v - expected).abs() < 1e-9, "{} = {}, expected {}", s, v, expected);
}

#[test]
fn exact_values_at_zero() {
    assert_eq!(run("erf(0);"), "0");
    assert_eq!(run("erfc(0);"), "1");
    assert_eq!(run("erfi(0);"), "0");
    assert_eq!(run("expintegral_si(0);"), "0");
    assert_eq!(run("fresnel_s(0);"), "0");
    assert_eq!(run("fresnel_c(0);"), "0");
}

#[test]
fn symbolic_args_stay_noun() {
    // integer (non-zero) and symbolic arguments are not floated
    assert_eq!(run("erf(1);"), "erf(1)");
    assert!(run("erfi(x);").contains("erfi"));
    assert!(run("expintegral_ei(x);").contains("expintegral_ei"));
}

#[test]
fn diff_rules() {
    assert_eq!(run("diff(expintegral_li(x), x);"), "1/log(x)");
    assert_eq!(run("diff(expintegral_si(x), x);"), "sin(x)/x");
    assert_eq!(run("diff(expintegral_ci(x), x);"), "cos(x)/x");
    assert_eq!(run("diff(expintegral_ei(x), x);"), "exp(x)/x");
    // erf/erfi derivatives contain the gaussian and 1/sqrt(%pi)
    let d_erf = run("diff(erf(x), x);");
    assert!(d_erf.contains("exp(-x^2)") && d_erf.contains("sqrt(%pi)"), "got: {}", d_erf);
    let d_erfi = run("diff(erfi(x), x);");
    assert!(d_erfi.contains("exp(x^2)") && d_erfi.contains("sqrt(%pi)"), "got: {}", d_erfi);
    // fresnel
    assert!(run("diff(fresnel_s(x), x);").contains("sin"));
    assert!(run("diff(fresnel_c(x), x);").contains("cos"));
}

#[test]
fn float_reference_values() {
    approx("float(erf(1));", 0.842_700_792_949_714_9);
    approx("erf(1.0);", 0.842_700_792_949_714_9);
    approx("erfi(1.0);", 1.650_425_758_797_542_8);
    approx("expintegral_ei(1.0);", 1.895_117_816_355_936_8);
    approx("expintegral_li(2.0);", 1.045_163_780_117_492_7);
    approx("expintegral_si(1.0);", 0.946_083_070_367_183_0);
    approx("expintegral_ci(1.0);", 0.337_403_922_900_968_1);
    approx("fresnel_s(1.0);", 0.438_259_147_390_354_8);
    approx("fresnel_c(1.0);", 0.779_893_400_376_822_8);
}
