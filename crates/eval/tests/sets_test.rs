use maxima_eval::eval_str;

fn run(s: &str) -> String { eval_str(s) }

#[test]
fn set_literal() {
    assert_eq!(run("{1,2,3};"), "{1,2,3}");
    assert_eq!(run("{3,1,2,1};"), "{1,2,3}");
    assert_eq!(run("{};"), "{}");
}

#[test]
fn set_assign() {
    assert_eq!(run("A:{1,2,3}; A;"), "{1,2,3}");
}

#[test]
fn setify_listify() {
    assert_eq!(run("setify([3,1,2,1]);"), "{1,2,3}");
    assert_eq!(run("listify({a,b,c});"), "[a,b,c]");
}

#[test]
fn union() {
    assert_eq!(run("union({1,2,3},{3,4,5});"), "{1,2,3,4,5}");
    assert_eq!(run("union({a},{b},{c});"), "{a,b,c}");
    assert_eq!(run("union({},{1,2});"), "{1,2}");
}

#[test]
fn intersection() {
    assert_eq!(run("intersection({1,2,3},{2,3,4});"), "{2,3}");
    assert_eq!(run("intersection({1,2},{3,4});"), "{}");
    assert_eq!(run("intersection({a,b,c},{b,c,d},{c,d,e});"), "{c}");
}

#[test]
fn setdifference() {
    assert_eq!(run("setdifference({1,2,3},{2});"), "{1,3}");
    assert_eq!(run("setdifference({1,2,3},{});"), "{1,2,3}");
    assert_eq!(run("setdifference({1,2},{1,2});"), "{}");
}

#[test]
fn symdifference() {
    assert_eq!(run("symdifference({1,2,3},{2,3,4});"), "{1,4}");
}

#[test]
fn elementp() {
    assert_eq!(run("elementp(2, {1,2,3});"), "true");
    assert_eq!(run("elementp(5, {1,2,3});"), "false");
}

#[test]
fn subsetp() {
    assert_eq!(run("subsetp({1,2},{1,2,3});"), "true");
    assert_eq!(run("subsetp({1,4},{1,2,3});"), "false");
    assert_eq!(run("subsetp({},{1,2});"), "true");
}

#[test]
fn disjointp() {
    assert_eq!(run("disjointp({1,2},{3,4});"), "true");
    assert_eq!(run("disjointp({1,2},{2,3});"), "false");
}

#[test]
fn cardinality() {
    assert_eq!(run("cardinality({a,b,c});"), "3");
    assert_eq!(run("cardinality({});"), "0");
}

#[test]
fn powerset() {
    let r = run("powerset({1,2});");
    assert!(r.contains("{1,2}") && r.contains("{1}") && r.contains("{2}") && r.contains("{}"),
        "got: {}", r);
}

#[test]
fn member_set() {
    assert_eq!(run("member(b, {a,b,c});"), "true");
    assert_eq!(run("member(d, {a,b,c});"), "false");
}
