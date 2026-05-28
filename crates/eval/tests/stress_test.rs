use maxima_eval::eval_str;

fn run(s: &str) -> String { eval_str(s) }

// ============================================================
// COMPREHENSIVE STRESS TESTS
// Based on known CAS pitfalls, edge cases, and tricky inputs
// ============================================================

// ==================== LIMITS ====================

// --- Indeterminate forms: 0/0 ---
#[test] fn lim_00_sinx_x() { assert_eq!(run("limit(sin(x)/x, x, 0);"), "1"); }
#[test] fn lim_00_tanx_x() { assert_eq!(run("limit(tan(x)/x, x, 0);"), "1"); }
#[test] fn lim_00_1_cos_x2() {
    // (1-cos(x))/x² → 1/2
    let r = run("limit((1-cos(x))/x^2, x, 0);");
    assert!(r == "1/2" || r == "2^(-1)", "got: {}", r);
}
#[test] fn lim_00_log1px_x() {
    // log(1+x)/x → 1
    let r = run("limit(log(1+x)/x, x, 0);");
    assert_eq!(r, "1", "got: {}", r);
}

// --- Indeterminate forms: ∞/∞ ---
#[test] fn lim_inf_inf_ratio() {
    assert_eq!(run("limit((2*x^3+x)/(x^3+5), x, inf);"), "2");
}
#[test] fn lim_inf_inf_exp_exp() {
    // exp(2x)/exp(x) = exp(x) → ∞
    let r = run("limit(exp(2*x)/exp(x), x, inf);");
    assert_eq!(r, "inf", "got: {}", r);
}

// --- Indeterminate forms: 0·∞ ---
#[test] fn lim_0inf_x_log_x_at_0() {
    // x*log(x) → 0 as x→0+
    assert_eq!(run("limit(x*log(x), x, 0);"), "0");
}
#[test] fn lim_0inf_x2_log_x() {
    // x²*log(x) → 0 as x→0+
    let r = run("limit(x^2*log(x), x, 0);");
    assert_eq!(r, "0", "got: {}", r);
}

// --- Indeterminate forms: ∞-∞ ---
#[test] fn lim_inf_minus_inf_rational() {
    // x - sqrt(x²+x) → -1/2 via conjugate
    // sqrt(x²+x) = x*sqrt(1+1/x) ≈ x*(1+1/(2x)) = x + 1/2
    // Actually this is tricky with sign: for x→+∞, x>0, so sqrt(x²+x)=x*sqrt(1+1/x)
    let r = run("limit(x-sqrt(x^2+x), x, inf);");
    // Expected: -1/2
    eprintln!("x-sqrt(x²+x): {}", r);
}

// --- Indeterminate forms: 0^0 ---
#[test] fn lim_0_0_x_x() {
    assert_eq!(run("limit(x^x, x, 0);"), "1");
}

// --- Indeterminate forms: ∞^0 ---
#[test] fn lim_inf_0_x_1overx() {
    // x^(1/x) → 1 as x→∞ (∞^0 form)
    let r = run("limit(x^(1/x), x, inf);");
    assert!(r == "1" || r.contains("1"), "got: {}", r);
}

// --- Known tricky limits ---
#[test] fn lim_nested_log() {
    // log(x)^2/x → 0
    assert_eq!(run("limit(log(x)^2/x, x, inf);"), "0");
}
#[test] fn lim_exp_dominates_power() {
    // exp(x)/x^1000 → inf
    assert_eq!(run("limit(exp(x)/x^1000, x, inf);"), "inf");
}
#[test] fn lim_negative_inf() {
    // exp(x) as x→-∞ → 0
    let r = run("limit(exp(x), x, minf);");
    assert_eq!(r, "0", "got: {}", r);
}

// ==================== INTEGRATION ====================

// --- Power rule edge cases ---
#[test] fn int_x_neg1() {
    // ∫ x^(-1) = log(x)
    let r = run("integrate(1/x, x);");
    assert!(r.contains("log"), "got: {}", r);
}
#[test] fn int_x_neg2() {
    // ∫ x^(-2) = -x^(-1)
    let r = run("integrate(1/x^2, x);");
    assert!(!r.contains("integrate"), "got: {}", r);
}
#[test] fn int_x_half() {
    // ∫ x^(1/2) = (2/3)*x^(3/2) — rational exponents
    let r = run("integrate(sqrt(x), x);");
    assert!(!r.contains("integrate"), "should solve sqrt(x), got: {}", r);
}

// --- Zero integrand ---
#[test] fn int_zero() {
    assert_eq!(run("integrate(0, x);"), "0");
}

// --- Constant ---
#[test] fn int_constant() {
    let r = run("integrate(5, x);");
    assert!(r.contains("5") && r.contains("x"), "got: {}", r);
}

// --- Composition depth ---
#[test] fn int_exp_exp() {
    // ∫ exp(exp(x))*exp(x) = exp(exp(x)) via substitution u=exp(x)
    let r = run("integrate(exp(exp(x))*exp(x), x);");
    assert!(!r.contains("integrate"), "should solve via subst, got: {}", r);
}
#[test] fn int_log_squared_over_x() {
    // ∫ log(x)^2/x = log(x)^3/3
    let r = run("integrate(log(x)^2/x, x);");
    assert!(!r.contains("integrate"), "got: {}", r);
}

// --- Trig identities under integration ---
#[test] fn int_sin_cos_same() {
    // ∫ sin(x)*cos(x) = sin²(x)/2 (or -cos²(x)/2)
    let r = run("integrate(sin(x)*cos(x), x);");
    assert!(!r.contains("integrate"), "got: {}", r);
}

// --- Rational with cancellation ---
#[test] fn int_x_over_x() {
    // ∫ (x²-1)/(x-1) = ∫ (x+1) = x²/2 + x
    let r = run("integrate((x^2-1)/(x-1), x);");
    assert!(!r.contains("integrate"), "should cancel and integrate, got: {}", r);
}

// --- Definite integration edge cases ---
#[test] fn defint_same_bounds() {
    // ∫_a^a f = 0
    assert_eq!(run("integrate(x^2, x, 3, 3);"), "0");
}
#[test] fn defint_negative_result() {
    // ∫_0^1 (x-1) = -1/2
    let r = run("integrate(x-1, x, 0, 1);");
    assert!(r == "-1/2" || r == "-2^(-1)", "got: {}", r);
}
#[test] fn defint_exp_decay() {
    // ∫_0^∞ exp(-2x) = 1/2
    let r = run("integrate(exp(-2*x), x, 0, inf);");
    assert!(r == "1/2" || r == "2^(-1)", "got: {}", r);
}

// ==================== SUMMATION ====================

// --- Edge cases ---
#[test] fn sum_empty_range() {
    // sum(k, k, 5, 3) — lo > hi → 0
    assert_eq!(run("sum(k, k, 5, 3);"), "0");
}
#[test] fn sum_single_term() {
    assert_eq!(run("sum(k, k, 7, 7);"), "7");
}

// --- Symbolic sums ---
#[test] fn sum_2k() {
    // Σ 2k = 2·n(n+1)/2 = n(n+1)
    let r = run("sum(2*k, k, 1, n);");
    assert!(!r.contains("sum"), "should simplify, got: {}", r);
}
#[test] fn sum_k_plus_1() {
    // Σ(k+1) from 1 to n = Σk + n = n(n+1)/2 + n
    let r = run("sum(k+1, k, 1, n);");
    assert!(!r.contains("sum"), "got: {}", r);
}
#[test] fn sum_3_k() {
    // Σ 3^k from 0 to n = (3^(n+1)-1)/2
    let r = run("sum(3^k, k, 0, n);");
    assert!(!r.contains("sum"), "geometric should work, got: {}", r);
}

// --- Numeric verification ---
#[test] fn sum_k_numeric_check() {
    // sum(k, k, 1, 100) = 5050
    assert_eq!(run("sum(k, k, 1, 100);"), "5050");
}
#[test] fn sum_k2_numeric_check() {
    // sum(k², k, 1, 10) = 385
    assert_eq!(run("sum(k^2, k, 1, 10);"), "385");
}
#[test] fn sum_geometric_numeric() {
    // sum(2^k, k, 0, 10) = 2047
    assert_eq!(run("sum(2^k, k, 0, 10);"), "2047");
}

// ==================== DIFFERENTIATION ====================

// --- Chain rule depth ---
#[test] fn diff_chain_exp_sin() {
    let r = run("diff(exp(sin(x)), x);");
    assert!(r.contains("cos") && r.contains("exp"), "got: {}", r);
}
#[test] fn diff_chain_log_x2p1() {
    // d/dx log(x²+1) = 2x/(x²+1)
    let r = run("diff(log(x^2+1), x);");
    assert!(r.contains("2") && r.contains("x"), "got: {}", r);
}

// --- Product rule ---
#[test] fn diff_product_x_sin() {
    // d/dx(x*sin(x)) = sin(x) + x*cos(x)
    let r = run("diff(x*sin(x), x);");
    assert!(r.contains("sin") && r.contains("cos"), "got: {}", r);
}

// --- Quotient rule ---
#[test] fn diff_quotient() {
    // d/dx(sin(x)/x) — should not panic
    let r = run("diff(sin(x)/x, x);");
    assert!(r.contains("cos") || r.contains("sin"), "got: {}", r);
}

// --- Higher derivatives ---
#[test] fn diff_second() {
    // d²/dx²(x³) = 6x
    let r = run("diff(x^3, x, 2);");
    assert!(r.contains("6") && r.contains("x"), "got: {}", r);
}
#[test] fn diff_third_sin() {
    // d³/dx³(sin(x)) = -cos(x)
    let r = run("diff(sin(x), x, 3);");
    assert!(r.contains("cos"), "got: {}", r);
}

// ==================== SIMPLIFIER ====================

#[test] fn simp_zero_add() { assert_eq!(run("0 + x;"), "x"); }
#[test] fn simp_zero_mul() { assert_eq!(run("0 * x;"), "0"); }
#[test] fn simp_one_mul() { assert_eq!(run("1 * x;"), "x"); }
#[test] fn simp_x_minus_x() { assert_eq!(run("x - x;"), "0"); }
#[test] fn simp_double_neg() { assert_eq!(run("-(-x);"), "x"); }
#[test] fn simp_power_zero() { assert_eq!(run("x^0;"), "1"); }
#[test] fn simp_power_one() { assert_eq!(run("x^1;"), "x"); }

// --- Pythagorean ---
#[test] fn simp_sin2_cos2() { assert_eq!(run("sin(x)^2 + cos(x)^2;"), "1"); }
#[test] fn simp_sin2_cos2_coeff() {
    // 3*sin²+3*cos² = 3
    let r = run("3*sin(x)^2 + 3*cos(x)^2;");
    assert_eq!(r, "3", "got: {}", r);
}

// ==================== ARITHMETIC ====================

#[test] fn arith_large_factorial() {
    // 20! = 2432902008176640000
    let r = run("20!;");
    assert!(r.contains("2432902008176640000"), "got: {}", r);
}
#[test] fn arith_rational_exact() {
    // 1/3 + 1/6 = 1/2
    let r = run("1/3 + 1/6;");
    assert!(r == "1/2" || r == "2^(-1)", "got: {}", r);
}
#[test] fn arith_power_negative() {
    // 2^(-3) = 1/8
    let r = run("2^(-3);");
    assert!(r == "1/8" || r == "8^(-1)", "got: {}", r);
}

// ==================== V3.3 LRT + V3.4 RADICAL DIAGNOSTICS ====================

#[test]
fn v3_diagnostic() {
    let cases = vec![
        ("integrate(1/(x^3+1), x);", "LRT: cubic denom"),
        ("integrate(1/(x^4+1), x);", "LRT: quartic denom"),
        ("integrate(1/sqrt(x^2+1), x);", "radical: asinh"),
        ("integrate(sqrt(1-x^2), x);", "radical: half circle"),
        ("integrate(1/(x*sqrt(x^2-1)), x);", "radical: acos(1/x)"),
        ("integrate(1/sqrt(x^2+2*x+2), x);", "radical: completing sq"),
        ("integrate(x/sqrt(x^2+1), x);", "radical: subst"),
        ("integrate((2*x+1)/sqrt(x^2+x+1), x);", "radical: deriv recog"),
    ];
    eprintln!("\n=== V3.3/V3.4 Diagnostic ===");
    let mut ok = 0;
    for (input, desc) in &cases {
        let r = run(input);
        let pass = !r.contains("integrate");
        if pass { ok += 1; }
        eprintln!("  [{}] {} => {}", if pass {"OK"} else {"--"}, desc, r);
    }
    eprintln!("  {}/{}\n", ok, cases.len());
}

#[test]
fn sum_binomial_via_zeilberger() {
    // Σ binomial(n,k) = 2^n — should work via Zeilberger when pattern fails
    let r = run("sum(binomial(n,k), k, 0, n);");
    assert!(r.contains("2") && r.contains("n"), "got: {}", r);
    assert!(!r.contains("sum"), "should resolve, got: {}", r);
}

#[test]
fn sum_2k_binomial_via_zeilberger() {
    // Σ 2^k·binomial(n,k) = 3^n
    let r = run("sum(2^k*binomial(n,k), k, 0, n);");
    assert!(!r.contains("sum"), "should resolve via Zeilberger, got: {}", r);
    // Verify numerically: for n=4, 3^4 = 81
    let v = run("sum(2^k*binomial(4,k), k, 0, 4);");
    assert_eq!(v, "81", "3^4=81, got: {}", v);
}

#[test]
fn sum_3k_binomial() {
    // Σ 3^k·binomial(n,k) = 4^n
    let v = run("sum(3^k*binomial(5,k), k, 0, 5);");
    assert_eq!(v, "1024", "4^5=1024, got: {}", v);
}

// ==================== CAUCHY PRINCIPAL VALUE ====================

#[test]
fn cauchy_pv_1_over_x() {
    // PV ∫_{-1}^{1} 1/x dx = 0 (odd function, symmetric interval)
    let r = run("integrate(1/x, x, -1, 1);");
    assert_eq!(r, "0", "PV of 1/x on [-1,1] should be 0, got: {}", r);
}

#[test]
fn cauchy_pv_x_over_x2() {
    // ∫_{-2}^{2} x/(x²+1) dx = 0 (odd integrand)
    let r = run("integrate(x/(x^2+1), x, -2, 2);");
    assert_eq!(r, "0", "got: {}", r);
}

// ==================== BINOMIAL + ZEILBERGER ====================

#[test]
fn binomial_numeric() {
    assert_eq!(run("binomial(5, 2);"), "10");
    assert_eq!(run("binomial(10, 3);"), "120");
    assert_eq!(run("binomial(0, 0);"), "1");
    assert_eq!(run("binomial(5, 0);"), "1");
    assert_eq!(run("binomial(5, 5);"), "1");
}

#[test]
fn binomial_edge() {
    assert_eq!(run("binomial(5, 6);"), "0");
    assert_eq!(run("binomial(5, -1);"), "0");
}

#[test]
fn sum_binomial_2n() {
    // Σ binomial(n,k) from 0 to n = 2^n
    let r = run("sum(binomial(n,k), k, 0, n);");
    assert!(!r.contains("sum"), "should recognize identity, got: {}", r);
    assert!(r.contains("2") && r.contains("n"), "got: {}", r);
}

#[test]
fn sum_binomial_numeric_check() {
    // sum(binomial(5,k), k, 0, 5) = 2^5 = 32
    assert_eq!(run("sum(binomial(5,k), k, 0, 5);"), "32");
}

#[test]
fn sum_alternating_binomial() {
    // Σ (-1)^k * binomial(n,k) from 0 to n = 0
    let r = run("sum((-1)^k*binomial(n,k), k, 0, n);");
    assert_eq!(r, "0", "alternating binomial should be 0, got: {}", r);
}

// ==================== EVALUATION ROBUSTNESS ====================

#[test] fn eval_nested_func() {
    // sin(cos(tan(1))) — should evaluate to a float
    let r = run("sin(cos(tan(1)));");
    assert!(r.contains(".") || r.contains("sin"), "got: {}", r);
}
#[test] fn eval_large_sum() {
    // sum(1, k, 1, 10000) = 10000 — tests iteration limit
    assert_eq!(run("sum(1, k, 1, 10000);"), "10000");
}
#[test] fn eval_deeply_nested() {
    // ((((x+1)+1)+1)+1) should simplify to x+4
    let r = run("((((x+1)+1)+1)+1);");
    assert!(r.contains("4") && r.contains("x"), "got: {}", r);
}
