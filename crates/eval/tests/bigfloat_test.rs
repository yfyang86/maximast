// Bundle 2 / 1c: arbitrary-precision bigfloat backend (astro-float). bfloat(x)
// evaluates the whole expression to fpprec digits; arithmetic mixing a bigfloat
// with other numbers folds at the widest operand precision ("contagion").
// Stored as a precision-tagged decimal in core, printed in Maxima `…bN` form.
use maxima_eval::{eval_str_with_env, Environment};

/// Run a sequence of statements in one env, return the LAST result (whitespace
/// stripped). Lets a test set fpprec then evaluate.
fn run_seq(stmts: &[&str]) -> String {
    let mut env = Environment::new();
    let mut out = String::new();
    for s in stmts {
        out = eval_str_with_env(s, &mut env);
    }
    out.split_whitespace().collect()
}

/// Parse a Maxima bigfloat string ("3.1415…b0") to f64 (b → e).
fn bf_f64(s: &str) -> f64 {
    s.replace('b', "e").parse::<f64>().unwrap_or_else(|_| panic!("not a bigfloat: {s}"))
}

/// Count significant digits in a "d.dddbN" string.
fn sig_digits(s: &str) -> usize {
    let mantissa = s.split('b').next().unwrap();
    mantissa.chars().filter(|c| c.is_ascii_digit()).count()
}

#[test]
fn pi_to_40_digits() {
    let r = run_seq(&["fpprec: 40;", "bfloat(%pi);"]);
    // π to 40 digits
    assert!(r.starts_with("3.14159265358979323846264338327950288419"), "got {r}");
    assert!(r.ends_with("b0"), "got {r}");
    assert_eq!(sig_digits(&r), 40);
}

#[test]
fn sqrt2_and_e() {
    let r = run_seq(&["fpprec: 30;", "bfloat(sqrt(2));"]);
    assert!(r.starts_with("1.41421356237309504880168872420"), "got {r}");
    let r = run_seq(&["fpprec: 30;", "bfloat(%e);"]);
    assert!(r.starts_with("2.71828182845904523536028747135"), "got {r}");
}

#[test]
fn precision_follows_fpprec() {
    assert_eq!(sig_digits(&run_seq(&["fpprec: 16;", "bfloat(%pi);"])), 16);
    assert_eq!(sig_digits(&run_seq(&["fpprec: 50;", "bfloat(%pi);"])), 50);
}

#[test]
fn integers_and_rationals() {
    assert_eq!(run_seq(&["fpprec: 20;", "bfloat(10);"]), "1.0b1");
    assert_eq!(run_seq(&["fpprec: 20;", "bfloat(0);"]), "0.0b0");
    assert_eq!(run_seq(&["fpprec: 20;", "bfloat(-2);"]), "-2.0b0");
    assert!((bf_f64(&run_seq(&["fpprec: 20;", "bfloat(1/3);"])) - 1.0 / 3.0).abs() < 1e-19);
}

#[test]
fn whole_expression_eval() {
    // the entire argument is evaluated at precision, including functions
    assert!((bf_f64(&run_seq(&["fpprec: 30;", "bfloat(sin(1));"])) - 1f64.sin()).abs() < 1e-15);
    assert!((bf_f64(&run_seq(&["fpprec: 30;", "bfloat(log(2));"])) - 2f64.ln()).abs() < 1e-15);
    assert!((bf_f64(&run_seq(&["fpprec: 30;", "bfloat(%pi + %e);"]))
        - (std::f64::consts::PI + std::f64::consts::E)).abs() < 1e-14);
}

#[test]
fn contagion_folds_arithmetic() {
    assert_eq!(run_seq(&["fpprec: 20;", "bfloat(2)*bfloat(3);"]), "6.0b0");
    assert!((bf_f64(&run_seq(&["fpprec: 20;", "bfloat(%pi)+1;"]))
        - (std::f64::consts::PI + 1.0)).abs() < 1e-18);
    assert!((bf_f64(&run_seq(&["fpprec: 20;", "bfloat(%pi)*2;"]))
        - 2.0 * std::f64::consts::PI).abs() < 1e-18);
    assert!((bf_f64(&run_seq(&["fpprec: 20;", "bfloat(%pi)^2;"]))
        - std::f64::consts::PI.powi(2)).abs() < 1e-17);
    // negative exponent (1/3 = mul(1, 3^-1))
    assert!((bf_f64(&run_seq(&["fpprec: 20;", "bfloat(1)/bfloat(3);"])) - 1.0 / 3.0).abs() < 1e-19);
}

#[test]
fn fpprec_query_and_default() {
    // fpprec() reports the default; once assigned, the bound variable reads back.
    assert_eq!(run_seq(&["fpprec();"]), "16");
    assert_eq!(run_seq(&["fpprec: 25;", "fpprec;"]), "25");
}
