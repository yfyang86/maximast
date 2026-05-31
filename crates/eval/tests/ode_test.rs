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
