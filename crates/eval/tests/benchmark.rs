use maxima_eval::eval_str;
use std::time::Instant;

fn run(s: &str) -> String { eval_str(s) }

fn timed(label: &str, input: &str) -> (String, f64) {
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(input)));
    let elapsed = start.elapsed().as_secs_f64();
    let output = result.unwrap_or_else(|_| "PANIC".to_string());
    (output, elapsed)
}

// ==================== CORRECTNESS BENCHMARKS ====================
// Verify AND time a wide range of operations

#[test]
fn bench_arithmetic() {
    let cases = vec![
        ("2+3;", "5"),
        ("100!;", ""), // just check it doesn't panic
        ("2^100;", ""),
        ("gcd(12345678, 87654321);", "9"),
        ("binomial(20, 10);", "184756"),
        ("1/3 + 1/6;", "1/2"),
        ("2^(-3);", "1/8"),
        ("(-3)^(-2);", "1/9"),
    ];
    eprintln!("\n=== Arithmetic ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = expected.is_empty() || r == *expected;
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, r);
        if !expected.is_empty() { assert_eq!(r, *expected, "input: {}", input); }
    }
}

#[test]
fn bench_float() {
    let cases = vec![
        ("float(%pi);", "3.141592653589793"),
        ("float(%e);", "2.718281828459045"),
        ("float(sqrt(2));", "1.414213562373095"),
        ("float(1/7);", "0.142857142857143"),
        ("float(sin(%pi/6));", "0.5"),
    ];
    eprintln!("\n=== Float ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = r == *expected || (r.len() >= 5 && expected.len() >= 5 && r.starts_with(&expected[..5]));
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, r);
        assert!(ok, "input: {} => got {} (expected {})", input, r, expected);
    }
}

#[test]
fn bench_algebra() {
    let cases = vec![
        ("expand((x+1)^6);", true),
        ("factor(x^6-1);", true),
        ("ratsimp((x^2-1)/(x-1));", true),
        ("gcd(x^4-1, x^6-1);", true),
        ("subst(3, x, x^3+2*x+1);", true),
        ("trigexpand(sin(a+b));", true),
        ("trigsimp(sin(x)^2+cos(x)^2);", true),
    ];
    eprintln!("\n=== Algebra ===");
    for (input, should_solve) in &cases {
        let (r, t) = timed("", input);
        let ok = !r.contains("PANIC");
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, r);
        assert!(ok);
    }
}

#[test]
fn bench_solve() {
    let cases = vec![
        ("solve(x^2-5*x+6, x);", true, ""),
        ("solve(x^2+3*x+2=0, x);", true, "[x = -1,x = -2]"),
        ("solve(a*x^2+b*x+c=0, x);", true, ""),
        ("solve(x^4-5*x^2+4, x);", true, ""),
        ("linsolve([x+y=3, 2*x-y=0], [x,y]);", true, ""),
    ];
    eprintln!("\n=== Solve ===");
    for (input, should_solve, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = if *should_solve { !r.starts_with("solve(") } else { true };
        let match_ok = expected.is_empty() || r == *expected;
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok && match_ok {"OK"} else {"FAIL"}, input, r);
        if *should_solve { assert!(!r.starts_with("solve("), "should solve: {} => {}", input, r); }
        if !expected.is_empty() { assert_eq!(r, *expected, "input: {}", input); }
    }
}

#[test]
fn bench_calculus() {
    let cases = vec![
        ("diff(sin(x^2), x);", false),
        ("diff(exp(x)*log(x), x);", false),
        ("diff(atan(x), x);", false),
        ("integrate(x^3, x);", false),
        ("integrate(sin(x)^2, x);", false),
        ("integrate(1/(x*(x+1)^2), x);", false),
        ("integrate(exp(x)*sin(x), x);", false),
        ("integrate(1/(x^4+1), x);", false),
        ("integrate(x*exp(x^2), x);", false),
        ("integrate(log(x)^3/x, x);", false),
        ("integrate(sqrt(1-x^2), x);", false),
        ("integrate(1/(x*log(x)), x);", false),
    ];
    eprintln!("\n=== Calculus ===");
    for (input, expect_noun) in &cases {
        let (r, t) = timed("", input);
        let has_noun = r.contains("integrate(") || r.contains("diff(");
        let ok = if *expect_noun { has_noun } else { !has_noun };
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, &r[..r.len().min(60)]);
        if !expect_noun { assert!(!has_noun, "should solve: {} => {}", input, r); }
    }
}

#[test]
fn bench_limits() {
    let cases = vec![
        ("limit(sin(x)/x, x, 0);", "1"),
        ("limit((1+1/x)^x, x, inf);", "exp(1)"),
        ("limit(exp(x)/x^100, x, inf);", "inf"),
        ("limit(log(x)/x, x, inf);", "0"),
        ("limit(x*sin(1/x), x, inf);", "1"),
        ("limit(exp(x+exp(-x))-exp(x), x, inf);", "1"),
        ("limit(sqrt(x^2+1)-x, x, inf);", "0"),
        ("limit((exp(x)-1-x)/x^2, x, 0);", "1/2"),
        ("limit(exp(x), x, minf);", "0"),
    ];
    eprintln!("\n=== Limits ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = r == *expected;
        eprintln!("  [{:.4}s] [{}] {} => {} (expect {})", t, if ok {"OK"} else {"FAIL"}, input, r, expected);
        assert_eq!(r, *expected, "input: {}", input);
    }
}

#[test]
fn bench_summation() {
    let cases = vec![
        ("sum(k, k, 1, 100);", "5050"),
        ("sum(k^2, k, 1, 10);", "385"),
        ("sum(k, k, 1, n);", ""),
        ("sum(2^k, k, 0, n);", ""),
        ("sum(1/(k*(k+1)), k, 1, n);", ""),
        ("sum(binomial(n,k), k, 0, n);", ""),
    ];
    eprintln!("\n=== Summation ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = if expected.is_empty() { !r.contains("sum(") } else { r == *expected };
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, r);
        if !expected.is_empty() { assert_eq!(r, *expected, "input: {}", input); }
        else { assert!(!r.contains("sum("), "should resolve: {} => {}", input, r); }
    }
}

#[test]
fn bench_definite_integrals() {
    let cases = vec![
        ("integrate(x^2, x, 0, 1);", "1/3"),
        ("integrate(sin(x), x, 0, %pi);", "2"),
        ("integrate(exp(-x), x, 0, inf);", "1"),
        ("integrate(1/(x^2+1), x, minf, inf);", "%pi"),
        ("integrate(x^3*exp(-x), x, 0, inf);", "6"),
        ("integrate(x^n*exp(-x), x, 0, inf);", "factorial(n)"),
    ];
    eprintln!("\n=== Definite Integrals ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = r == *expected;
        eprintln!("  [{:.4}s] [{}] {} => {} (expect {})", t, if ok {"OK"} else {"FAIL"}, input, r, expected);
        assert_eq!(r, *expected, "input: {}", input);
    }
}

#[test]
fn bench_matrix() {
    let cases = vec![
        ("determinant(matrix([1,2],[3,4]));", "-2"),
        ("determinant(matrix([a,b],[c,d]));", ""),
        ("charpoly(matrix([1,2],[3,4]), x);", ""),
        ("transpose(matrix([1,2,3],[4,5,6]));", ""),
    ];
    eprintln!("\n=== Matrix ===");
    for (input, expected) in &cases {
        let (r, t) = timed("", input);
        let ok = expected.is_empty() || r == *expected;
        eprintln!("  [{:.4}s] [{}] {} => {}", t, if ok {"OK"} else {"FAIL"}, input, r);
        if !expected.is_empty() { assert_eq!(r, *expected); }
    }
}

#[test]
fn bench_performance_scaling() {
    eprintln!("\n=== Performance Scaling ===");
    // Polynomial expansion: (x+1)^n
    for n in [10, 20, 50] {
        let input = format!("expand((x+1)^{});", n);
        let (_, t) = timed("", &input);
        eprintln!("  expand((x+1)^{}): {:.4}s", n, t);
        assert!(t < 5.0, "expand((x+1)^{}) too slow: {}s", n, t);
    }
    // Factoring
    for p in ["x^4-1", "x^6-1", "x^8-1", "x^12-1"] {
        let input = format!("factor({});", p);
        let (_, t) = timed("", &input);
        eprintln!("  factor({}): {:.4}s", p, t);
    }
    // GCD
    for d in [4, 8, 12] {
        let input = format!("gcd(x^{}-1, x^{}-1);", d, d + 2);
        let (_, t) = timed("", &input);
        eprintln!("  gcd(x^{}-1, x^{}-1): {:.4}s", d, d + 2, t);
    }
    // Sum iteration
    for n in [100, 1000, 5000] {
        let input = format!("sum(k^2, k, 1, {});", n);
        let (_, t) = timed("", &input);
        eprintln!("  sum(k^2, k, 1, {}): {:.4}s", n, t);
        assert!(t < 5.0, "sum too slow for n={}: {}s", n, t);
    }
}
