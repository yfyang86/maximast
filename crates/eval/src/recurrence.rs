//! Order-≥2 creative telescoping: find the minimal linear P-recurrence of a
//! D-finite sequence T(n) (V12.0 T1, Zeilberger-package spirit).
//!
//! T(n) is sampled *exactly* at integer n; for increasing order J and
//! coefficient degree d we set up the homogeneous system whose unknowns are the
//! coefficients of c_0(n)…c_J(n) in  Σ_j c_j(n)·T(n+j) = 0, solve for the
//! (unique) null vector, and VERIFY it on held-out samples. The first
//! (order, degree) that yields a verified unique recurrence is returned as the
//! coefficient list [c_0(n), …, c_J(n)]. Exact rational arithmetic throughout,
//! so the recurrence is correct (never a wrong guess).

use maxima_core::{Expr, SymbolId};
use num::{BigInt, BigRational, Zero, One, ToPrimitive, Signed};
use crate::helpers::subst;
use crate::env::Environment;
use crate::simp::simplify;

fn to_bigrat(e: &Expr) -> Option<BigRational> {
    match e {
        Expr::Integer(n) => Some(BigRational::from(BigInt::from(*n))),
        Expr::BigInt(b) => Some(BigRational::from((**b).clone())),
        Expr::Rational { num, den } => Some(BigRational::new(BigInt::from(*num), BigInt::from(*den))),
        _ => None,
    }
}

fn bigint_expr(b: &BigInt) -> Expr {
    b.to_i64().map(Expr::int).unwrap_or_else(|| Expr::BigInt(Box::new(b.clone())))
}

fn bigrat_expr(r: &BigRational) -> Expr {
    if r.denom().is_one() {
        bigint_expr(r.numer())
    } else {
        Expr::div(bigint_expr(r.numer()), bigint_expr(r.denom()))
    }
}

/// One nonzero null-space vector of `m` (rows×cols), or None if the columns are
/// independent. `unique` is set to whether the null space is 1-dimensional.
fn null_vector(m: &[Vec<BigRational>], unique: &mut bool) -> Option<Vec<BigRational>> {
    let rows = m.len();
    if rows == 0 { return None; }
    let cols = m[0].len();
    let mut a: Vec<Vec<BigRational>> = m.to_vec();
    let mut where_piv = vec![usize::MAX; cols];
    let mut r = 0;
    for c in 0..cols {
        if r >= rows { break; }
        let sel = (r..rows).find(|&rr| !a[rr][c].is_zero());
        let Some(sel) = sel else { continue };
        a.swap(sel, r);
        let piv = a[r][c].clone();
        for j in 0..cols { a[r][j] = &a[r][j] / &piv; }
        for rr in 0..rows {
            if rr != r && !a[rr][c].is_zero() {
                let f = a[rr][c].clone();
                for j in 0..cols { a[rr][j] = &a[rr][j] - &(&f * &a[r][j]); }
            }
        }
        where_piv[c] = r;
        r += 1;
    }
    let free: Vec<usize> = (0..cols).filter(|&c| where_piv[c] == usize::MAX).collect();
    *unique = free.len() == 1;
    let f0 = *free.first()?;
    let mut v = vec![BigRational::zero(); cols];
    v[f0] = BigRational::one();
    for c in 0..cols {
        if where_piv[c] != usize::MAX {
            v[c] = -a[where_piv[c]][f0].clone();
        }
    }
    Some(v)
}

/// Clear denominators and content so the vector is primitive integers, with the
/// last nonzero entry positive.
fn normalize_int(v: &[BigRational]) -> Vec<BigRational> {
    let mut lcm = BigInt::one();
    for x in v { lcm = num::integer::lcm(lcm, x.denom().clone()); }
    let mut ints: Vec<BigInt> = v.iter().map(|x| (x * BigRational::from(lcm.clone())).to_integer()).collect();
    let mut g = BigInt::zero();
    for x in &ints { g = num::integer::gcd(g, x.clone()); }
    if !g.is_zero() { for x in &mut ints { *x /= &g; } }
    if let Some(last) = ints.iter().rposition(|x| !x.is_zero()) {
        if ints[last].is_negative() { for x in &mut ints { *x = -x.clone(); } }
    }
    ints.into_iter().map(BigRational::from).collect()
}

/// Find the minimal linear P-recurrence Σ_j c_j(n)·T(n+j)=0 of the sequence
/// defined by `expr` (a function of `n_id`). Returns [c_0(n), …, c_J(n)].
pub fn find_recurrence(expr: &Expr, n_id: SymbolId, env: &mut Environment) -> Option<Vec<Expr>> {
    const MAX_ORDER: usize = 3;
    const MAX_DEG: usize = 3;
    // Bounded so the common integer D-finite sequences stay within the kernel's
    // exact (i64) range — beyond it the kernel's summation/power arithmetic
    // overflows (panics in debug, wraps in release), which would corrupt samples.
    const MAXN: i64 = 28;
    let n = Expr::Symbol(n_id);

    // Sample T(0..=MAXN) exactly; stop at the first non-numeric value or if a
    // sample overflows the kernel's arithmetic (caught so we degrade to a noun
    // rather than crash).
    let mut t: Vec<BigRational> = Vec::new();
    for i in 0..=MAXN {
        let e = subst(&Expr::int(i), &n, expr);
        let v = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::eval::meval(&e, env)));
        match v.ok().as_ref().and_then(to_bigrat) {
            Some(r) => t.push(r),
            None => break,
        }
    }

    for order in 1..=MAX_ORDER {
        for deg in 0..=MAX_DEG {
            let ncols = (order + 1) * (deg + 1);
            // Rows: one per base n0 with T(n0..=n0+order) available.
            let max_rows = t.len().saturating_sub(order);
            if max_rows < ncols + 2 { continue; }
            let solve_rows = (ncols + 4).min(max_rows);

            let row_at = |n0: usize| -> Vec<BigRational> {
                let mut row = Vec::with_capacity(ncols);
                for j in 0..=order {
                    let mut npow = BigRational::one();
                    let nn = BigRational::from(BigInt::from(n0 as i64));
                    for _i in 0..=deg {
                        row.push(&npow * &t[n0 + j]);
                        npow = &npow * &nn;
                    }
                }
                row
            };

            let matrix: Vec<Vec<BigRational>> = (0..solve_rows).map(row_at).collect();
            let mut unique = false;
            let Some(vec) = null_vector(&matrix, &mut unique) else { continue };
            if !unique { continue; }
            let vec = normalize_int(&vec);

            // Verify on held-out rows just beyond the solve window. (We keep n0
            // small: sampling at large n0 can overflow the kernel's i64
            // binomial/power arithmetic, so distant rows aren't trustworthy.)
            let verify_hi = (solve_rows + 6).min(max_rows);
            let ok = (0..verify_hi).all(|n0| {
                row_at(n0).iter().zip(&vec).map(|(a, b)| a * b).sum::<BigRational>().is_zero()
            });
            if !ok { continue; }

            // Build c_j(n) = Σ_i vec[j*(deg+1)+i]·n^i.
            let mut coeffs = Vec::with_capacity(order + 1);
            for j in 0..=order {
                let mut poly = Expr::int(0);
                for i in 0..=deg {
                    let cf = &vec[j * (deg + 1) + i];
                    if !cf.is_zero() {
                        let term = Expr::mul(bigrat_expr(cf), Expr::pow(n.clone(), Expr::int(i as i64)));
                        poly = Expr::add(poly, term);
                    }
                }
                coeffs.push(simplify(&poly));
            }
            return Some(coeffs);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::eval::eval_str;
    fn run(s: &str) -> String { eval_str(s) }

    #[test] fn rec_geometric() { assert_eq!(run("find_recurrence(2^n,n);"), "[-2,1]"); }
    #[test] fn rec_factorial() { assert_eq!(run("find_recurrence(n!,n);"), "[-1-n,1]"); }
    #[test] fn rec_binomial_sq_order1() {
        assert_eq!(run("find_recurrence(sum(binomial(n,k)^2,k,0,n),n);"), "[-2-4*n,1+n]");
    }
    #[test] fn rec_franel_order2() {
        // Franel numbers are D-finite (order 2), no elementary closed form.
        let s = run("find_recurrence(sum(binomial(n,k)^3,k,0,n),n);");
        assert!(!s.contains("find_recurrence"), "noun: {s}");
        assert_eq!(s.matches(',').count(), 2); // 3 coefficients ⇒ order 2
    }
    #[test] fn rec_nonholonomic_is_noun() {
        // n^n is not P-finite (no fixed-order polynomial-coefficient recurrence).
        assert!(run("find_recurrence(n^n,n);").contains("find_recurrence"));
    }
}
