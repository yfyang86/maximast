// Basic list operations — regression tests after the rest(L, n) off-by-one fix.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn rest_default_drops_one() {
    assert_eq!(run("rest([10, 20, 30, 40]);"), "[20,30,40]");
}

#[test] fn rest_positive_n_drops_first_n() {
    // Pre-fix this returned [20,30,40] (always dropped 1).
    assert_eq!(run("rest([10, 20, 30, 40], 2);"), "[30,40]");
    assert_eq!(run("rest([10, 20, 30, 40], 3);"), "[40]");
}

#[test] fn rest_negative_n_drops_last_abs_n() {
    assert_eq!(run("rest([10, 20, 30, 40], -1);"), "[10,20,30]");
    assert_eq!(run("rest([10, 20, 30, 40], -2);"), "[10,20]");
}

#[test] fn rest_zero_n_is_identity() {
    assert_eq!(run("rest([10, 20, 30, 40], 0);"), "[10,20,30,40]");
}

#[test] fn rest_oversized_n_returns_empty() {
    assert_eq!(run("rest([10, 20], 99);"), "[]");
    assert_eq!(run("rest([10, 20], -99);"), "[]");
}

#[test] fn rest_on_empty_list() {
    assert_eq!(run("rest([]);"), "[]");
    assert_eq!(run("rest([], 3);"), "[]");
}

#[test] fn first_last_length() {
    assert_eq!(run("first([10, 20, 30]);"), "10");
    assert_eq!(run("last([10, 20, 30]);"), "30");
    assert_eq!(run("length([10, 20, 30, 40, 50]);"), "5");
}

// ---- list indexing via L[i] (was returning mqapply noun) ----

#[test] fn list_index_at_top_level() {
    assert_eq!(run("L : [10, 20, 30, 40]$ L[1];"), "10");
    assert_eq!(run("L : [10, 20, 30, 40]$ L[3];"), "30");
}

#[test] fn list_index_negative_counts_from_end() {
    assert_eq!(run("L : [10, 20, 30, 40]$ L[-1];"), "40");
    assert_eq!(run("L : [10, 20, 30, 40]$ L[-2];"), "30");
}

#[test] fn list_index_out_of_range_is_noun() {
    let r = run("L : [10, 20]$ L[5];");
    assert!(r.contains("mqapply") || r.contains("L[5]"), "got: {}", r);
}

#[test] fn list_index_inside_function_body() {
    // Pre-fix this stayed mqapply([100,200,300], 2) and produced no value.
    assert_eq!(run("f(L) := L[2]$ f([100, 200, 300]);"), "200");
}

#[test] fn list_index_inside_makelist() {
    // makelist over an L[i] body — the natural way to iterate a list.
    assert_eq!(run("g(L) := makelist(L[i], i, 1, length(L))$ g([1, 2, 3, 4]);"), "[1,2,3,4]");
}
