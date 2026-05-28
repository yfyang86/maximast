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
