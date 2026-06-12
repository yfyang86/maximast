// Regression: makelist used to eagerly evaluate its body in the outer scope
// (before binding the loop var), which caused infinite recursion in any
// recursive call whose argument depended on the loop var.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn makelist_basic() {
    assert_eq!(run("makelist(i, i, 1, 5);"), "[1,2,3,4,5]");
    assert_eq!(run("makelist(i*i, i, 1, 4);"), "[1,4,9,16]");
}

#[test] fn makelist_with_body_using_outer_var() {
    // Body references the (function-local) parameter L; loop var is i.
    assert_eq!(run("h(L) := makelist(part(L, i), i, 1, length(L))$ h([10, 20, 30]);"), "[10,20,30]");
}

#[test] fn recursive_makelist_body_with_loop_var_in_arg() {
    // Pre-fix: stack overflow. The eager outer-scope evaluation called the
    // recursive function with `rest(L, i)` where i is unbound, producing a
    // noun and looping forever before makelist itself could run.
    let r = run("rc(L) := if emptyp(L) then 0 else makelist(rc(rest(L, i)), i, 1, 1)$ rc([1]);");
    assert_eq!(r, "[0]");
}

#[test] fn natural_recursive_perms() {
    // The textbook recursive permutations definition — used to hang.
    let prog = "remove_at(L, i) := append(makelist(part(L,j), j, 1, i-1), \
                                          makelist(part(L,j), j, i+1, length(L)))$ \
                perms(L) := if emptyp(L) then [[]] \
                            else apply(append, \
                                       makelist(map(lambda([p], cons(part(L,i), p)), \
                                                    perms(remove_at(L, i))), \
                                                i, 1, length(L)))$ \
                length(perms([1,2,3,4]));";
    assert_eq!(run(prog), "24");
}
