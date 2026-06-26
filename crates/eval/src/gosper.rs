//! Gosper's algorithm — indefinite hypergeometric summation (A=B, Ch. 5).
//!
//! Given a hypergeometric term t(k) (i.e. t(k+1)/t(k) is a rational function of
//! k), Gosper's algorithm decides whether Σ t(k) has a hypergeometric closed
//! form and, if so, finds an antidifference T(k) with T(k+1) − T(k) = t(k).
//!
//! Every result is VERIFIED by telescoping before being returned, so the
//! algorithm is never wrong — at worst it returns None (not Gosper-summable, or
//! beyond our bounds), in which case callers fall back to the noun form.

use maxima_core::{Expr, Operator, SymbolId};
use maxima_poly::{Poly, Coeff, expr_to_cre, poly_to_expr, poly_gcd};
use crate::helpers::subst;
use crate::simp::simplify;

fn contains_k(e: &Expr, k_id: SymbolId) -> bool {
    match e {
        Expr::Symbol(id) => *id == k_id,
        Expr::List { args, .. } => args.iter().any(|a| contains_k(a, k_id)),
        _ => false,
    }
}

/// True if the expression contains a factorial or binomial anywhere — those
/// need the recursive shift-ratio path (the generic ratio can't reduce them).
fn has_factorial_or_binomial(e: &Expr) -> bool {
    match e {
        Expr::List { op: Operator::Named(id), args, .. } => {
            let n = maxima_core::resolve(*id);
            n == "factorial" || n == "binomial" || args.iter().any(has_factorial_or_binomial)
        }
        Expr::List { args, .. } => args.iter().any(has_factorial_or_binomial),
        _ => false,
    }
}

fn as_small_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Integer(n) if n.abs() <= 16 => Some(*n),
        _ => None,
    }
}

/// Hypergeometric shift ratio t(k+1)/t(k), computed structurally so that
/// exponentials a^(linear k) and factorials (which the generic simplifier does
/// not reduce) collapse to a rational function of k. Returns None if t is not
/// recognisably hypergeometric.
fn hyper_ratio(t: &Expr, k: &Expr, k_id: SymbolId) -> Option<Expr> {
    let k1 = Expr::add(k.clone(), Expr::int(1));
    if !contains_k(t, k_id) {
        return Some(Expr::int(1));
    }
    match t {
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut prod = Expr::int(1);
            for a in args { prod = Expr::mul(prod, hyper_ratio(a, k, k_id)?); }
            Some(simplify(&prod))
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let (base, exp) = (&args[0], &args[1]);
            if contains_k(base, k_id) {
                if matches!(exp, Expr::Integer(_)) {
                    if has_factorial_or_binomial(base) {
                        // factorial/binomial base: the generic subst-and-divide
                        // can't reduce e.g. (k−3)!^(−1); recurse so it becomes
                        // 1/(k−2) and binomials are Gosper-summable.
                        let br = hyper_ratio(base, k, k_id)?;
                        Some(simplify(&Expr::pow(br, exp.clone())))
                    } else {
                        // (poly/rational)^integer ⇒ the generic ratio is already
                        // a clean rational function of k.
                        Some(simplify(&Expr::div(subst(&k1, k, t), t.clone())))
                    }
                } else { None }
            } else {
                // a^e(k): ratio = a^(e(k+1) − e(k)); valid iff that exponent is k-free
                let delta = simplify(&Expr::sub(subst(&k1, k, exp), exp.clone()));
                if contains_k(&delta, k_id) { None }
                else { Some(simplify(&Expr::pow(base.clone(), delta))) }
            }
        }
        Expr::List { op: Operator::Named(id), args, .. }
            if *id == maxima_core::intern("factorial") && args.len() == 1 =>
        {
            let arg = &args[0];
            let slope = as_small_int(&simplify(&Expr::sub(subst(&k1, k, arg), arg.clone())))?;
            if slope >= 1 {
                let mut p = Expr::int(1);
                for i in 1..=slope { p = Expr::mul(p, Expr::add(arg.clone(), Expr::int(i))); }
                Some(simplify(&p))
            } else if slope <= -1 {
                let mut p = Expr::int(1);
                for i in 0..(-slope) { p = Expr::mul(p, Expr::sub(arg.clone(), Expr::int(i))); }
                Some(simplify(&Expr::div(Expr::int(1), p)))
            } else {
                Some(Expr::int(1))
            }
        }
        // binomial(a,b) = a!/(b!·(a−b)!): reduce to the factorial form so its
        // k-shift ratio is rational (e.g. binomial(k,m) → (k+1)/(k+1−m),
        // binomial(n+k,k) → (n+k+1)/(k+1)). Reuses the factorial arm above.
        Expr::List { op: Operator::Named(id), args, .. }
            if *id == maxima_core::intern("binomial") && args.len() == 2 =>
        {
            let (a, b) = (&args[0], &args[1]);
            let expanded = Expr::div(
                Expr::call("factorial", vec![a.clone()]),
                Expr::mul(
                    Expr::call("factorial", vec![b.clone()]),
                    Expr::call("factorial", vec![simplify(&Expr::sub(a.clone(), b.clone()))]),
                ),
            );
            hyper_ratio(&expanded, k, k_id)
        }
        // Plain polynomial / rational in k (incl. the symbol k itself, sums).
        _ => Some(simplify(&Expr::div(subst(&k1, k, t), t.clone()))),
    }
}

/// Degree of a polynomial as i64 (−1 for the zero polynomial).
fn deg(p: &Poly) -> i64 {
    p.degree().map(|d| d as i64).unwrap_or(-1)
}

/// Coefficient of k^e (zero if absent or e<0).
fn coeff_of(p: &Poly, e: i64) -> Coeff {
    if e < 0 { return Coeff::zero(); }
    let e = e as u32;
    p.terms.iter().find(|(x, _)| *x == e).map(|(_, c)| c.clone()).unwrap_or_else(Coeff::zero)
}

fn monomial(var: SymbolId, e: u32) -> Poly {
    Poly { var, terms: vec![(e, Coeff::one())] }
}

/// p(k + c), computed directly via polynomial arithmetic: Σ_e a_e·(k+c)^e.
fn poly_shift(p: &Poly, c: i64, k_id: SymbolId) -> Poly {
    if c == 0 { return p.clone(); }
    let kc = Poly { var: k_id, terms: vec![(1, Coeff::one()), (0, Coeff::Int(c))] }; // k + c
    let mut result = Poly::zero(k_id);
    for (e, coeff) in &p.terms {
        let mut pw = Poly::constant(k_id, Coeff::one());
        for _ in 0..*e { pw = pw.mul(&kc); }
        result = result.add(&pw.scale(coeff));
    }
    result
}

/// Gosper–Petkovšek normal form: write num/den = (a/b)·(c(k+1)/c(k)) with
/// gcd(a(k), b(k+h)) = 1 for all integers h ≥ 0.
fn gosper_petkovsek(num: &Poly, den: &Poly, k_id: SymbolId) -> Option<(Poly, Poly, Poly)> {
    let mut a = num.clone();
    let mut b = den.clone();
    let mut c = Poly::constant(k_id, Coeff::one());
    const H_MAX: i64 = 64; // dispersion search bound (verification gates correctness)
    for h in 1..=H_MAX {
        let b_sh = poly_shift(&b, h, k_id);          // b(k+h)
        let g = poly_gcd(&a, &b_sh);                 // common factor p(k)
        if g.is_constant() { continue; }
        a = a.exact_div(&g)?;                        // a /= p(k)
        let g_neg = poly_shift(&g, -h, k_id);        // p(k-h) | b(k)
        b = b.exact_div(&g_neg)?;                    // b /= p(k-h)
        for i in 1..=h {                             // c *= ∏_{i=1}^{h} p(k-i)
            c = c.mul(&poly_shift(&g, -i, k_id));
        }
    }
    Some((a, b, c))
}

/// Solve a(k)·x(k+1) − bsh(k)·x(k) = c(k) for a polynomial x(k), where
/// bsh(k) = b(k−1). Returns the polynomial solution if one exists.
fn solve_gosper_equation(a: &Poly, bsh: &Poly, c: &Poly, k_id: SymbolId) -> Option<Poly> {
    let (la, lb, dc) = (deg(a), deg(bsh), deg(c));
    // Degree bound for x.
    let dx: i64 = if la != lb {
        dc - la.max(lb)
    } else {
        let lca = a.leading_coeff();
        let lcb = bsh.leading_coeff();
        if !lca.sub(&lcb).is_zero() {
            dc - la
        } else {
            // Equal degree & equal leading coeff: the k^d·(x(k+1)−x(k)) leading
            // terms cancel, so x's degree is generically deg(c) − d + 1; plus a
            // special candidate D0 = (B1 − A1)/lc when that is a nonneg integer.
            let a1 = coeff_of(a, la - 1);
            let b1 = coeff_of(bsh, lb - 1);
            let d0 = b1.sub(&a1).div(&lca);
            let d0i = match d0 {
                Some(Coeff::Int(n)) if n >= 0 => n,
                _ => -1,
            };
            (dc - la + 1).max(d0i)
        }
    };
    if dx < 0 || dx > 256 { return None; }
    let dx = dx as u32;

    // Unknowns u_0..u_dx. For each i, the known polynomial
    //   P_i(k) = a(k)·(k+1)^i − bsh(k)·k^i
    // contributes u_i·P_i; we need Σ_i u_i P_i = c.
    let mut p_cols: Vec<Poly> = Vec::with_capacity(dx as usize + 1);
    let mut maxdeg = dc;
    for i in 0..=dx {
        let ki = monomial(k_id, i);
        let ki1 = poly_shift(&ki, 1, k_id);          // (k+1)^i
        let pi = a.mul(&ki1).sub(&bsh.mul(&ki));
        maxdeg = maxdeg.max(deg(&pi));
        p_cols.push(pi);
    }
    if maxdeg < 0 { return None; }
    let ncols = dx as usize + 1;
    // Row per power 0..=maxdeg.
    let mut mat: Vec<Vec<Coeff>> = Vec::new();
    let mut rhs: Vec<Coeff> = Vec::new();
    for power in 0..=maxdeg {
        let mut row = Vec::with_capacity(ncols);
        for col in &p_cols {
            row.push(coeff_of(col, power));
        }
        mat.push(row);
        rhs.push(coeff_of(c, power));
    }
    let sol = solve_particular(mat, rhs)?;
    // Build x(k) = Σ u_i k^i.
    let mut terms: Vec<(u32, Coeff)> = Vec::new();
    for (i, u) in sol.iter().enumerate() {
        if !u.is_zero() { terms.push((i as u32, u.clone())); }
    }
    terms.sort_by(|x, y| y.0.cmp(&x.0));
    Some(Poly { var: k_id, terms })
}

/// Gaussian elimination returning *a particular* rational solution (free
/// variables set to 0), or None only if the system is inconsistent. Gosper's
/// key equation is typically underdetermined (the antidifference's free
/// additive constant), so we must accept free variables rather than demand a
/// unique solution.
fn solve_particular(mut mat: Vec<Vec<Coeff>>, mut b: Vec<Coeff>) -> Option<Vec<Coeff>> {
    let nrows = mat.len();
    if nrows == 0 { return None; }
    let ncols = mat[0].len();
    let mut where_pivot = vec![usize::MAX; ncols];
    let mut pivot_row = 0;
    for col in 0..ncols {
        if pivot_row >= nrows { break; }
        let sel = (pivot_row..nrows).find(|&r| !mat[r][col].is_zero());
        let Some(sel) = sel else { continue };
        mat.swap(sel, pivot_row);
        b.swap(sel, pivot_row);
        where_pivot[col] = pivot_row;
        let piv = mat[pivot_row][col].clone();
        for j in 0..ncols { mat[pivot_row][j] = mat[pivot_row][j].div(&piv)?; }
        b[pivot_row] = b[pivot_row].div(&piv)?;
        for r in 0..nrows {
            if r != pivot_row && !mat[r][col].is_zero() {
                let f = mat[r][col].clone();
                for j in 0..ncols { mat[r][j] = mat[r][j].sub(&f.mul(&mat[pivot_row][j])); }
                b[r] = b[r].sub(&f.mul(&b[pivot_row]));
            }
        }
        pivot_row += 1;
    }
    for r in 0..nrows {
        if mat[r].iter().all(|c| c.is_zero()) && !b[r].is_zero() { return None; }
    }
    let mut sol = vec![Coeff::zero(); ncols];
    for col in 0..ncols {
        let pr = where_pivot[col];
        if pr != usize::MAX { sol[col] = b[pr].clone(); }
    }
    Some(sol)
}

/// Indefinite hypergeometric sum: antidifference T(k) with T(k+1)−T(k)=t(k).
pub fn gosper_sum(t: &Expr, k: &Expr) -> Option<Expr> {
    let k_id = if let Expr::Symbol(id) = k { *id } else { return None; };

    // r(k) = t(k+1)/t(k) as reduced num/den.
    let ratio = simplify(&crate::eval::ratsimp_pub(&hyper_ratio(t, k, k_id)?));
    let cre = expr_to_cre(&ratio, k_id)?;
    if cre.num.is_zero() { return None; }

    let (a, b, c) = gosper_petkovsek(&cre.num, &cre.den, k_id)?;
    let bsh = poly_shift(&b, -1, k_id);              // b(k−1)
    let x = solve_gosper_equation(&a, &bsh, &c, k_id)?;
    if x.is_zero() { return None; }

    // T(k) = (b(k−1)·x(k) / c(k)) · t(k).
    let rt = bsh.mul(&x);
    let r_expr = Expr::div(poly_to_expr(&rt), poly_to_expr(&c));
    let big_t = simplify(&Expr::mul(r_expr, t.clone()));

    // Verify by telescoping. Gosper is exact by construction; this guards
    // against implementation bugs and handles non-rational terms (factorials)
    // that a symbolic zero-test wouldn't cancel. Check T(k+1)−T(k)−t(k) ≈ 0 at
    // several integer points.
    if telescopes(&big_t, t, k) { Some(big_t) } else { None }
}

/// Gosper/WZ certificate of an indefinite hypergeometric sum: the rational
/// R(k) with antidifference T(k) = R(k)·t(k), i.e. t(k) = T(k+1) − T(k). The
/// certifying identity is  R(k+1)·r(k) − R(k) = 1  where r(k) = t(k+1)/t(k);
/// it is checked SYMBOLICALLY (a rigorous proof of Σ t(k) = T(b+1) − T(a)),
/// with a numeric telescoping fall-back for terms a symbolic zero-test can't
/// cancel (factorials).
pub fn gosper_certificate(t: &Expr, k: &Expr) -> Option<Expr> {
    let k_id = if let Expr::Symbol(id) = k { *id } else { return None; };
    let ratio = simplify(&crate::eval::ratsimp_pub(&hyper_ratio(t, k, k_id)?));
    let cre = expr_to_cre(&ratio, k_id)?;
    if cre.num.is_zero() { return None; }

    let (a, b, c) = gosper_petkovsek(&cre.num, &cre.den, k_id)?;
    let bsh = poly_shift(&b, -1, k_id);
    let x = solve_gosper_equation(&a, &bsh, &c, k_id)?;
    if x.is_zero() { return None; }

    // Certificate R(k) = b(k−1)·x(k) / c(k).
    let r_cert = simplify(&Expr::div(poly_to_expr(&bsh.mul(&x)), poly_to_expr(&c)));

    // Verify R(k+1)·r(k) − R(k) − 1 = 0. Try symbolic (rigorous) first.
    let k1 = Expr::add(k.clone(), Expr::int(1));
    let r_shift = subst(&k1, k, &r_cert);
    let residual = Expr::sub(Expr::sub(Expr::mul(r_shift, ratio.clone()), r_cert.clone()), Expr::int(1));
    let simp = simplify(&crate::eval::ratsimp_pub(&residual));
    if matches!(simp, Expr::Integer(0)) {
        return Some(r_cert);
    }
    // Numeric fall-back: T(k) = R(k)·t(k) must telescope to t(k).
    let big_t = simplify(&Expr::mul(r_cert.clone(), t.clone()));
    if telescopes(&big_t, t, k) { Some(r_cert) } else { None }
}

/// Definite hypergeometric sum Σ_{k=lo}^{hi} t(k) = T(hi+1) − T(lo).
pub fn gosper_definite(t: &Expr, k: &Expr, lo: &Expr, hi: &Expr) -> Option<Expr> {
    let big_t = gosper_sum(t, k)?;
    let upper = subst(&Expr::add(hi.clone(), Expr::int(1)), k, &big_t);
    let lower = subst(lo, k, &big_t);
    Some(simplify(&crate::eval::ratsimp_pub(&Expr::sub(upper, lower))))
}

/// Numerically check that T(k+1) − T(k) = t(k) at several integer points.
fn telescopes(big_t: &Expr, t: &Expr, k: &Expr) -> bool {
    let k1 = Expr::add(k.clone(), Expr::int(1));
    let diff = Expr::sub(Expr::sub(subst(&k1, k, big_t), big_t.clone()), t.clone());
    let mut env = crate::Environment::new();
    let mut checked = 0;
    for ki in 2..=11i64 {
        let at = subst(&Expr::int(ki), k, &diff);
        let v = crate::eval::meval(&at, &mut env);
        match crate::helpers::to_f64(&v) {
            Some(x) => {
                if x.abs() > 1e-6 { return false; }
                checked += 1;
            }
            None => {} // not numerically evaluable here; skip this point
        }
    }
    checked >= 4
}

#[cfg(test)]
mod tests {
    use crate::eval::eval_str;

    fn run(s: &str) -> String { eval_str(s) }

    #[test] fn gosper_polynomial() {
        // Σ_{k=1}^n k = n(n+1)/2; check numerically via the closed form at n=10 → 55.
        assert_eq!(run("sum(k,k,1,10);"), "55");
        // symbolic closed form exists (not a noun)
        assert!(!run("nusum(k^2,k,1,n);").contains("nusum"));
    }

    #[test] fn gosper_exponential() {
        // Σ_{k=1}^n 2^k = 2^(n+1) − 2; at n=5 → 62.
        assert_eq!(run("nusum(2^k,k,1,5);"), "62");
    }

    #[test] fn gosper_factorial() {
        // Σ_{k=1}^n k·k! = (n+1)! − 1; at n=4 → 119.
        assert_eq!(run("nusum(k*k!,k,1,4);"), "119");
    }

    #[test] fn gosper_rational_telescoping() {
        // Σ_{k=1}^n 1/(k(k+1)) = 1 − 1/(n+1); at n=3 → 3/4.
        assert_eq!(run("nusum(1/(k*(k+1)),k,1,3);"), "3/4");
    }

    #[test] fn gosper_cert_factorial() {
        // T(k)=k!, certificate R(k)=1/k (k*k! = (k+1)!-k!).
        assert_eq!(run("gosper_certificate(k*k!,k);"), "1/k");
    }
    #[test] fn gosper_cert_geometric() {
        assert_eq!(run("gosper_certificate(2^k,k);"), "1");
    }
    #[test] fn gosper_cert_not_summable_is_noun() {
        assert!(run("gosper_certificate(1/k,k);").contains("gosper_certificate"));
    }
    #[test] fn gosper_not_summable_is_noun() {
        // 1/k^2 is not Gosper-summable → stays a noun (never a wrong closed form).
        assert!(run("nusum(1/k^2,k,1,n);").contains("nusum"));
    }
    #[test] fn gosper_binomial_hockey_stick() {
        // Σ_{k=0}^n binomial(k,m) = binomial(n+1,m+1) — binomials now reduce to
        // factorials in the shift ratio, so Gosper telescopes them.
        assert_eq!(run("subst(5, n, nusum(binomial(k,3), k, 0, n));"), "15"); // binomial(6,4)
        assert_eq!(run("subst(7, n, nusum(binomial(k,2), k, 0, n));"), "56"); // binomial(8,3)
        // WZ certificate of binomial(k,2): R(k)=(k-2)/3 (T(k)=binomial(k,3)).
        assert!(!run("gosper_certificate(binomial(k,2), k);").contains("gosper_certificate"));
    }
}
