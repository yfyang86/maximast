// S5: advanced trig — trigrat, extended trigreduce, halfangles.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

// ---------- trigrat ----------

#[test] fn trigrat_double_angle() {
    assert_eq!(run("trigrat(2*sin(x)*cos(x));"), "sin(2*x)");
}
#[test] fn trigrat_sin_sq() {
    assert_eq!(norm(&run("trigrat(sin(x)^2);")), "(1-cos(2*x))/2");
}
#[test] fn trigrat_cos_sq() {
    assert_eq!(norm(&run("trigrat(cos(x)^2);")), "(1+cos(2*x))/2");
}
#[test] fn trigrat_pythagorean() {
    assert_eq!(run("trigrat(sin(x)^2+cos(x)^2);"), "1");
}

// ---------- Extended trigreduce: higher powers ----------

#[test] fn trigreduce_sin_cubed() {
    // sin^3 x = (3 sin x - sin 3x)/4
    let r = norm(&run("trigreduce(sin(x)^3);"));
    assert!(r.contains("3*sin(x)") && r.contains("sin(3*x)") && r.contains("/4"), "got: {}", r);
}
#[test] fn trigreduce_cos_cubed() {
    let r = norm(&run("trigreduce(cos(x)^3);"));
    assert!(r.contains("3*cos(x)") && r.contains("cos(3*x)") && r.contains("/4"), "got: {}", r);
}
#[test] fn trigreduce_sin_fourth() {
    let r = norm(&run("trigreduce(sin(x)^4);"));
    assert!(r.contains("cos(2*x)") && r.contains("cos(4*x)") && r.contains("/8"), "got: {}", r);
}

// ---------- Extended trigreduce: product-to-sum (different angles) ----------

#[test] fn trigreduce_sin_a_cos_b() {
    // sin(a)cos(b) = (sin(a+b) + sin(a-b))/2
    let r = norm(&run("trigreduce(sin(a)*cos(b));"));
    assert!(r.contains("sin(a+b)") && r.contains("sin(a-b)") && r.contains("/2"), "got: {}", r);
}
#[test] fn trigreduce_sin_a_sin_b() {
    // sin(a)sin(b) = (cos(a-b) - cos(a+b))/2
    let r = norm(&run("trigreduce(sin(a)*sin(b));"));
    assert!(r.contains("cos(a-b)") && r.contains("cos(a+b)") && r.contains("/2"), "got: {}", r);
}
#[test] fn trigreduce_cos_a_cos_b() {
    let r = norm(&run("trigreduce(cos(a)*cos(b));"));
    assert!(r.contains("cos(a-b)") && r.contains("cos(a+b)") && r.contains("/2"), "got: {}", r);
}

// ---------- halfangles ----------

#[test] fn halfangles_sin() {
    // sin(x/2) -> sqrt((1-cos x)/2)
    let r = norm(&run("halfangles(sin(x/2));"));
    assert!(r.contains("sqrt") && r.contains("1-cos(x)"), "got: {}", r);
}
#[test] fn halfangles_cos() {
    let r = norm(&run("halfangles(cos(x/2));"));
    assert!(r.contains("sqrt") && r.contains("1+cos(x)"), "got: {}", r);
}
#[test] fn halfangles_tan() {
    // tan(x/2) -> sin(x)/(1+cos(x))
    let r = norm(&run("halfangles(tan(x/2));"));
    assert!(r.contains("sin(x)") && r.contains("1+cos(x)"), "got: {}", r);
}

// ---------- Regression: ordinary fraction display unaffected ----------

#[test] fn fraction_display_unaffected() {
    // The trigrat numeric-factor merge must not disturb normal X/n display.
    assert_eq!(run("integrate(x/(x^2+1), x);"), "log(1+x^2)/2");
    assert_eq!(norm(&run("trigreduce(sin(x)^2);")), "(1-cos(2*x))/2");
}
