use maxima_eval::eval_str;

// ============================================================
// V3.2 STRESS TESTS — verify Gruntz MRV algorithm is general,
// not hardcoded for specific expressions
// ============================================================

fn run(input: &str) -> String {
    eval_str(input)
}

// --- Variations of the classic exp(x+exp(-x))-exp(x) ---

#[test]
fn gruntz_classic_original() {
    // THE classic: exp(x+exp(-x))-exp(x) → 1
    assert_eq!(run("limit(exp(x+exp(-x))-exp(x), x, inf);"), "1");
}

#[test]
fn gruntz_classic_scaled() {
    // exp(x+2*exp(-x))-exp(x) → 2 (the coefficient scales)
    let r = run("limit(exp(x+2*exp(-x))-exp(x), x, inf);");
    assert_eq!(r, "2", "got: {}", r);
}

#[test]
fn gruntz_classic_3() {
    // exp(x+3*exp(-x))-exp(x) → 3
    let r = run("limit(exp(x+3*exp(-x))-exp(x), x, inf);");
    assert_eq!(r, "3", "got: {}", r);
}

// --- Basic exp dominance variants ---

#[test]
fn exp_over_x() { assert_eq!(run("limit(exp(x)/x, x, inf);"), "inf"); }

#[test]
fn exp_over_x3() { assert_eq!(run("limit(exp(x)/x^3, x, inf);"), "inf"); }

#[test]
fn exp_over_x50() { assert_eq!(run("limit(exp(x)/x^50, x, inf);"), "inf"); }

#[test]
fn exp_neg_times_x() { assert_eq!(run("limit(x*exp(-x), x, inf);"), "0"); }

#[test]
fn exp_neg_times_x5() { assert_eq!(run("limit(x^5*exp(-x), x, inf);"), "0"); }

// --- Log-based limits ---

#[test]
fn log_over_x() { assert_eq!(run("limit(log(x)/x, x, inf);"), "0"); }

#[test]
fn log_over_sqrt_x() {
    let r = run("limit(log(x)/sqrt(x), x, inf);");
    assert_eq!(r, "0", "log/√x → 0, got: {}", r);
}

#[test]
fn log_log_over_log() {
    assert_eq!(run("limit(log(log(x))/log(x), x, inf);"), "0");
}

// --- sqrt conjugate variants ---

#[test]
fn sqrt_x2_plus_1_minus_x() {
    assert_eq!(run("limit(sqrt(x^2+1)-x, x, inf);"), "0");
}

#[test]
fn sqrt_x2_plus_4_minus_x() {
    // sqrt(x²+4)-x = 4/(sqrt(x²+4)+x) → 0
    let r = run("limit(sqrt(x^2+4)-x, x, inf);");
    assert_eq!(r, "0", "got: {}", r);
}

// --- 0*∞ indeterminate forms ---

#[test]
fn x_sin_1_over_x() {
    assert_eq!(run("limit(x*sin(1/x), x, inf);"), "1");
}

#[test]
fn x_tan_1_over_x() {
    // x*tan(1/x) → 1 (same as sin(1/x)/(1/x))
    let r = run("limit(x*tan(1/x), x, inf);");
    assert_eq!(r, "1", "got: {}", r);
}

// --- 1^∞ forms ---

#[test]
fn classic_e_limit() {
    let r = run("limit((1+1/x)^x, x, inf);");
    assert!(r == "exp(1)" || r == "%e", "got: {}", r);
}

#[test]
fn e_squared_limit() {
    // (1+2/x)^x → e²
    let r = run("limit((1+2/x)^x, x, inf);");
    assert!(r.contains("exp") && r.contains("2"), "expected exp(2), got: {}", r);
}

// --- Finite L'Hopital variants ---

#[test]
fn lhopital_sin_x_over_x() {
    assert_eq!(run("limit(sin(x)/x, x, 0);"), "1");
}

#[test]
fn lhopital_exp_minus_1_over_x() {
    assert_eq!(run("limit((exp(x)-1)/x, x, 0);"), "1");
}

#[test]
fn lhopital_second_order() {
    // (exp(x)-1-x)/x² → 1/2
    let r = run("limit((exp(x)-1-x)/x^2, x, 0);");
    assert!(r == "1/2" || r == "2^(-1)", "got: {}", r);
}

#[test]
fn lhopital_third_order() {
    // (exp(x)-1-x-x²/2)/x³ → 1/6
    let r = run("limit((exp(x)-1-x-x^2/2)/x^3, x, 0);");
    assert!(r == "1/6" || r == "6^(-1)", "got: {}", r);
}

// --- Nested exponentials ---

#[test]
fn exp_exp_to_inf() {
    assert_eq!(run("limit(exp(exp(x)), x, inf);"), "inf");
}

#[test]
fn exp_neg_exp_to_zero() {
    assert_eq!(run("limit(exp(-exp(x)), x, inf);"), "0");
}

// --- Rational function limits at infinity ---

#[test]
fn rational_same_degree() {
    let r = run("limit((3*x^2+2*x+1)/(x^2+5), x, inf);");
    assert_eq!(r, "3", "got: {}", r);
}

#[test]
fn rational_higher_denom() {
    assert_eq!(run("limit((x+1)/(x^2+1), x, inf);"), "0");
}
