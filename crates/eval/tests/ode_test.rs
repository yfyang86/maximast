use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// First-order ODEs
#[test] fn ode2_separable_simple() {
    let r = run("ode2('diff(y,x)=x, y, x);");
    assert!(r.contains("y") && r.contains("x^2") && r.contains("%c"), "got: {}", r);
}

#[test] fn ode2_linear_first() {
    let r = run("ode2('diff(y,x)+y=0, y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
}

#[test] fn ode2_separable_xy() {
    let r = run("ode2('diff(y,x)=x*y, y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
}

// Second-order constant-coefficient
#[test] fn ode2_second_distinct_real() {
    // y'' - y = 0 → exp(x), exp(-x)
    let r = run("ode2('diff(y,x,2)-y=0, y, x);");
    assert!(r.contains("exp") && r.contains("%k1") && r.contains("%k2"), "got: {}", r);
}

#[test] fn ode2_second_complex() {
    // y'' + y = 0 → cos(x), sin(x)
    let r = run("ode2('diff(y,x,2)+y=0, y, x);");
    assert!(r.contains("cos") && r.contains("sin"), "got: {}", r);
}

#[test] fn ode2_second_repeated() {
    // y'' + 4y' + 4y = 0 → (k1+k2*x)*exp(-2x)
    let r = run("ode2('diff(y,x,2)+4*'diff(y,x)+4*y=0, y, x);");
    assert!(r.contains("exp") && r.contains("%k1") && r.contains("%k2"), "got: {}", r);
}

#[test] fn ode2_second_damped() {
    // y'' + 2y' + 5y = 0 → complex roots: exp(-x)(cos(2x), sin(2x))
    let r = run("ode2('diff(y,x,2)+2*'diff(y,x)+5*y=0, y, x);");
    assert!(r.contains("cos") && r.contains("sin") && r.contains("exp"), "got: {}", r);
}

// --- S6: non-homogeneous (undetermined coefficients, general polynomial) ---
#[test] fn ode2_nonhom_poly() {
    // y'' + y = x^2 → particular x^2 - 2
    let r = run("ode2('diff(y,x,2)+y=x^2, y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
    assert!(r.contains("x^2") && r.contains("%k1") && r.contains("%k2"), "got: {}", r);
}

#[test] fn ode2_nonhom_poly_linear() {
    // y'' - 3y' + 2y = x → particular x/2 + 3/4
    let r = run("ode2('diff(y,x,2)-3*'diff(y,x)+2*y=x, y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
}

#[test] fn ode2_nonhom_exp_resonance() {
    // y'' - y = exp(x): exp(x) is a homogeneous solution → x*exp(x)/2
    let r = run("ode2('diff(y,x,2)-y=exp(x), y, x);");
    assert!(!r.contains("ode2"), "should solve, got: {}", r);
    assert!(r.contains("x*exp(x)") || r.contains("exp(x)*x"), "got: {}", r);
}

// --- S6: variation of parameters (resonant trig forcing) ---
#[test] fn ode2_nonhom_resonant_sin() {
    // y'' + y = sin(x): resonance, undetermined coeffs fails, VOP solves it.
    let r = run("ode2('diff(y,x,2)+y=sin(x), y, x);");
    assert!(!r.contains("ode2"), "should solve via variation of parameters, got: {}", r);
}

// --- S6: initial / boundary conditions ---
#[test] fn ic1_first_order() {
    let r = run("ic1(ode2('diff(y,x)=x, y, x), x=0, y=1);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("1") && n.contains("x^2") && !n.contains("%c"), "got: {}", r);
}

#[test] fn ic2_cos_solution() {
    // y'' + y = 0, y(0)=1, y'(0)=0 → cos(x)
    let r = run("ic2(ode2('diff(y,x,2)+y=0, y, x), x=0, y=1, 'diff(y,x)=0);");
    assert_eq!(r.chars().filter(|c| !c.is_whitespace()).collect::<String>(), "y=cos(x)");
}

#[test] fn ic2_sin_solution() {
    // y'' + y = 0, y(0)=0, y'(0)=1 → sin(x)
    let r = run("ic2(ode2('diff(y,x,2)+y=0, y, x), x=0, y=0, 'diff(y,x)=1);");
    assert_eq!(r.chars().filter(|c| !c.is_whitespace()).collect::<String>(), "y=sin(x)");
}

#[test] fn bc2_two_points() {
    // y'' + y = 0, y(0)=0, y(pi/2)=1 → sin(x)
    let r = run("bc2(ode2('diff(y,x,2)+y=0, y, x), x=0, y=0, x=%pi/2, y=1);");
    assert_eq!(r.chars().filter(|c| !c.is_whitespace()).collect::<String>(), "y=sin(x)");
}

#[test] fn ic2_nonhomogeneous() {
    // y'' + y = x^2, y(0)=0, y'(0)=0 → x^2 - 2 + 2*cos(x)
    let r = run("ic2(ode2('diff(y,x,2)+y=x^2, y, x), x=0, y=0, 'diff(y,x)=0);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("x^2") && n.contains("cos(x)") && !n.contains("%k"), "got: {}", r);
}

// Euler–Cauchy (variable-coefficient) equations, V13 3f.
#[test] fn euler_distinct_real_roots() {
    // x^2 y'' + x y' - y = 0 → m^2 - 1 = 0 → y = k1 x + k2/x
    let r = run("ode2('x^2*'diff(y,x,2)+x*'diff(y,x)-y=0, y, x);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("%k1*x") && n.contains("%k2/x"), "got: {}", r);
}
#[test] fn euler_repeated_root() {
    // x^2 y'' - 3x y' + 4y = 0 → (m-2)^2 → y = x^2 (k1 + k2 ln x)
    let r = run("ode2('x^2*'diff(y,x,2)-3*x*'diff(y,x)+4*y=0, y, x);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("x^2") && n.contains("log(x)"), "got: {}", r);
}
#[test] fn euler_complex_roots() {
    // x^2 y'' + x y' + y = 0 → m^2 + 1 = 0 → y = k1 cos(ln x) + k2 sin(ln x)
    let r = run("ode2('x^2*'diff(y,x,2)+x*'diff(y,x)+y=0, y, x);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("cos(log(x))") && n.contains("sin(log(x))"), "got: {}", r);
}

// desolve via the Laplace method, V13 3g.
#[test] fn desolve_first_order_symbolic_ic() {
    assert_eq!(run("desolve('diff(y,t)=y, y(t));"), "y(t) = exp(t)*y(0)");
}
#[test] fn desolve_second_order_symbolic_ic() {
    // general solution y(0)cos t + y'(0) sin t
    let r = run("desolve('diff(y,t,2)+y=0, y(t));");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("cos(t)*y(0)") && n.contains("sin(t)"), "got: {}", r);
}
#[test] fn desolve_sinh_cosh() {
    // y''-y=0 → cosh/sinh (real poles, not complex cos)
    let r = run("desolve('diff(y,t,2)-y=0, y(t));");
    assert!(!r.contains("sqrt(-1)") && r.contains("exp(t)") && r.contains("exp(-t)"), "got: {}", r);
}
#[test] fn desolve_with_atvalue() {
    let mut env = maxima_eval::Environment::new();
    maxima_eval::eval_str_with_env("atvalue(y(t), t=0, 2);", &mut env);
    maxima_eval::eval_str_with_env("atvalue('diff(y,t), t=0, 3);", &mut env);
    let r = maxima_eval::eval_str_with_env("desolve('diff(y,t,2)+y=0, y(t));", &mut env);
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("2*cos(t)") && n.contains("3*sin(t)") && !n.contains("y(0)"), "got: {}", r);
}

// desolve for 2×2 first-order linear constant-coefficient systems (V13 3g+),
// via Laplace on the system. Output in terms of x(0),y(0).
#[test] fn desolve_system_cosh_sinh() {
    // x'=y, y'=x → x=cosh·x0+sinh·y0, y=sinh·x0+cosh·y0 (eigenvalues ±1)
    let r = run("desolve([diff(x(t),t)=y(t), diff(y(t),t)=x(t)], [x(t),y(t)]);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("exp(t)") && n.contains("exp(-t)") && n.contains("x(0)") && n.contains("y(0)"), "got: {}", r);
    assert!(n.starts_with("[x(t)=") && n.contains(",y(t)="), "shape: {}", r);
}
#[test] fn desolve_system_rotation() {
    // x'=-y, y'=x → complex eigenvalues ±i → cos/sin
    let r = run("desolve([diff(x(t),t)=-y(t), diff(y(t),t)=x(t)], [x(t),y(t)]);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert_eq!(n, "[x(t)=cos(t)*x(0)-sin(t)*y(0),y(t)=cos(t)*y(0)+sin(t)*x(0)]");
}
#[test] fn desolve_system_repeated_eigenvalue() {
    // x'=x, y'=x+y → repeated λ=1 → the t·exp(t) term appears
    let r = run("desolve([diff(x(t),t)=x(t), diff(y(t),t)=x(t)+y(t)], [x(t),y(t)]);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("t*exp(t)*x(0)"), "expected repeated-root term, got: {}", r);
}
#[test] fn desolve_system_with_atvalue() {
    let mut env = maxima_eval::Environment::new();
    maxima_eval::eval_str_with_env("atvalue(x(t), t=0, 1);", &mut env);
    maxima_eval::eval_str_with_env("atvalue(y(t), t=0, 0);", &mut env);
    let r = maxima_eval::eval_str_with_env(
        "desolve([diff(x(t),t)=y(t), diff(y(t),t)=x(t)], [x(t),y(t)]);", &mut env);
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    // x(0)=1,y(0)=0 → x=cosh(t), y=sinh(t); no symbolic x(0)/y(0) left.
    assert!(!n.contains("x(0)") && !n.contains("y(0)"), "ICs not applied: {}", r);
    assert!(n.contains("exp(t)") && n.contains("exp(-t)"), "got: {}", r);
}
#[test] fn desolve_system_forcing() {
    // x'=-x+1, y'=y → x has steady state 1: x = 1 + exp(-t)(x0-1)
    let r = run("desolve([diff(x(t),t)=-x(t)+1, diff(y(t),t)=y(t)], [x(t),y(t)]);");
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert!(n.contains("1+exp(-t)*x(0)-exp(-t)"), "forcing not handled: {}", r);
}
