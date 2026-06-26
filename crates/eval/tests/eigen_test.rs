// Bundle 2 / 3b: general eigenvalues + eigenvectors. Eigenvalues come from the
// charpoly factored over Q and solved by radical (rational, irrational,
// complex). Eigenvectors are an exact basis of null(M − λI): the divide-based
// RREF for rational λ, falling back to an adjugate column for radical λ (the
// RREF leaves an unreducible 1/λ residue the simplifier can't kill, but
// cofactors are polynomial in λ and reduce under expand). correct-or-noun.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn eigenvalues_rational() {
    // diagonal → its diagonal entries
    assert_eq!(run("eigenvalues(matrix([2,0],[0,3]));"), "[[2,3],[1,1]]");
    // symmetric integer → 1, 3
    assert_eq!(run("eigenvalues(matrix([2,1],[1,2]));"), "[[1,3],[1,1]]");
}

#[test]
fn eigenvalues_irrational_golden_ratio() {
    // charpoly x²−x−1 → (1±√5)/2, written via √(5/4)
    let r = run("eigenvalues(matrix([0,1],[1,1]));");
    assert_eq!(r, "[[sqrt(5/4)+1/2,-sqrt(5/4)+1/2],[1,1]]");
}

#[test]
fn eigenvalues_complex() {
    // rotation by 90° → ±i, no noun
    let r = run("eigenvalues(matrix([0,-1],[1,0]));");
    assert!(r.contains("%i") && !r.contains("eigenvalues"), "got: {}", r);
}

#[test]
fn eigenvectors_rational_diagonal() {
    // standard basis vectors
    assert_eq!(
        run("eigenvectors(matrix([2,0],[0,3]));"),
        "[[[2,3],[1,1]],[[matrix([1],[0])],[matrix([0],[1])]]]"
    );
}

#[test]
fn eigenvectors_irrational_nonempty() {
    // golden-ratio matrix: each radical eigenvalue must yield an eigenvector
    // (genuine eigenvalues are never eigenvector-free). Adjugate fallback gives
    // [1−λ, −1] for M − λI = [[−λ,1],[1,1−λ]].
    let r = run("eigenvectors(matrix([0,1],[1,1]));");
    assert!(!r.contains("eigenvectors"), "expected closed form, got: {}", r);
    assert!(r.contains("matrix"), "expected eigenvectors, got: {}", r);
    // both eigenvalues present, no empty eigenvector list
    assert!(r.contains("sqrt(5/4)+1/2"), "got: {}", r);
    assert!(!r.contains("[],[]"), "empty eigenvector basis, got: {}", r);
}

#[test]
fn eigenvectors_complex_nonempty() {
    let r = run("eigenvectors(matrix([0,-1],[1,0]));");
    assert!(!r.contains("eigenvectors"), "expected closed form, got: {}", r);
    assert!(r.contains("%i") && r.contains("matrix"), "got: {}", r);
}

#[test]
fn eigenvectors_defective_single_vector() {
    // Jordan block [[1,1],[0,1]]: eigenvalue 1 (mult 2), geometric mult 1.
    let r = run("eigenvectors(matrix([1,1],[0,1]));");
    assert_eq!(r, "[[[1],[2]],[[matrix([1],[0])]]]");
}
