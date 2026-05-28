use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn multthru_product_sum() { assert_eq!(run("multthru(a*(b+c));"), "a*b+a*c"); }
#[test] fn multthru_frac() {
    let r = run("multthru((a+b)/c);");
    assert!(r.contains("a") && r.contains("b") && r.contains("c"), "got: {}", r);
}
#[test] fn multthru_numeric() { assert_eq!(run("multthru(3*(x+2));"), "6+3*x"); }

#[test] fn xthru_sum_fracs() {
    let r = run("xthru(a/b + c/d);");
    assert!(!r.contains("+") || r.contains("("), "should be single fraction, got: {}", r);
}

#[test] fn collectterms_basic() {
    let r = run("collectterms(a*x+b*x+c, x);");
    assert!(r.contains("a+b") || r.contains("b+a"), "got: {}", r);
}

#[test] fn at_list() { assert_eq!(run("at(x^2+y, [x=3, y=1]);"), "10"); }
#[test] fn at_single() {
    let r = run("at(x^2+y, x=3);");
    assert!(r.contains("9") && r.contains("y"), "got: {}", r);
}
#[test] fn at_trig() {
    let r = run("at(sin(x)+1, x=0);");
    assert_eq!(r, "1");
}

#[test] fn lopow_poly() { assert_eq!(run("lopow(x^3+x, x);"), "1"); }
#[test] fn lopow_constant() { assert_eq!(run("lopow(x^3+1, x);"), "0"); }

#[test] fn lfreeof_true() { assert_eq!(run("lfreeof([x,y], a+b);"), "true"); }
#[test] fn lfreeof_false() { assert_eq!(run("lfreeof([x], x+1);"), "false"); }
