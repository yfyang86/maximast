// Regression: simplify_times silently dropped a Rational coefficient whenever
// a Float was also present in the product. Both arrived through correct input
// paths but only one of {num_prod, rat_num, float_prod} was emitted, so the
// other coefficient vanished — e.g. (1/2)*2.0 returned 2 (the float dropped
// the 1/2) instead of 1.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// Float 1.0 / 2.0 / etc. display as "1" / "2" / etc. in this kernel — the
// numeric value is what matters; the underlying Expr is still a Float.
#[test] fn rational_times_float_folds() {
    assert_eq!(run("(1/2) * 2.0;"), "1");
    assert_eq!(run("(1/3) * 6.0;"), "2");
    assert_eq!(run("(2/3) * 9.0;"), "6");
    assert_eq!(run("(3/2) * 4.0;"), "6");
    assert_eq!(run("(1/4) * 8.0;"), "2");
}

#[test] fn float_times_rational_folds() {
    assert_eq!(run("1.0 * (1/2);"), "0.5");
    assert_eq!(run("4.0 * (1/2);"), "2");
    assert_eq!(run("1.5 * (1/3);"), "0.5");
}

#[test] fn float_div_integer_folds() {
    assert_eq!(run("1.0 / 2;"), "0.5");
    assert_eq!(run("3.0 / 6;"), "0.5");
}

#[test] fn rational_times_symbol_unchanged() {
    // The pure-symbolic case must keep working: the rational stays exact.
    assert_eq!(run("(1/2) * x;"), "(1/2)*x");
}

#[test] fn rational_times_float_times_symbol() {
    // Pre-fix: "x" (both numerics lost or one of them). Post-fix: a single
    // folded float coefficient times x.
    assert_eq!(run("(1/2) * 4.0 * x;"), "2*x");
}

#[test] fn three_way_mix() {
    // Integer 3, Rational 1/6, Float 2.0 → 1.0.
    assert_eq!(run("3 * (1/6) * 2.0;"), "1");
}

// The closing bracket on the regression: ∫sin(x)/cos(x) etc. still equal -log(cos(x)) etc.
// And legendre_q (which produces (1/2)*log_term*P_n products) gets numerically right values.
