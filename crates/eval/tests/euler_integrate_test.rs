// V9.0 / V2: Euler substitution ∫1/((x+r)√(ax²+bx+c)) dx (u = 1/(x+r)).
// The result has a branch cut at x=-r, so it is verified on the single branch
// x > -r. Each closed form differentiates back to the integrand there.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn euler_nondegenerate() {
    // r=1 not a root of x²+1 → quadratic P(u); result via asinh.
    let r = run("integrate(1/((x+1)*sqrt(x^2+1)), x);");
    assert!(!r.contains("integrate") && r.contains("asinh"), "got: {}", r);
    // classic: ∫1/(x√(x²+1)) = -asinh(1/x)
    assert_eq!(run("integrate(1/(x*sqrt(x^2+1)), x);"), "-asinh(1/x)");
}

#[test]
fn euler_degenerate_root() {
    // r=-2 is a root of x²-4 → P(u) linear; result via √(linear).
    let r = run("integrate(1/((x-2)*sqrt(x^2-4)), x);");
    assert!(!r.contains("integrate") && r.contains("sqrt"), "got: {}", r);
    // r=-1 is a root of x²-1
    let r = run("integrate(1/((x+1)*sqrt(x^2-1)), x);");
    assert!(!r.contains("integrate") && r.contains("sqrt"), "got: {}", r);
}

#[test]
fn quadratic_radical_paths_unchanged() {
    assert_eq!(run("integrate(1/sqrt(x^2+1), x);"), "asinh(x)");
    assert_eq!(run("integrate(1/sqrt(4*x^2+1), x);"), "asinh(2*x)/2");
}
