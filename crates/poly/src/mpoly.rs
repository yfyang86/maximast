//! Sparse multivariate polynomial type, the foundation for V8.0's
//! polynomial-systems work (Gröbner bases, system solving, elimination).
//!
//! Terms are stored sorted descending under a chosen `MonomialOrder`
//! (Lex / Grlex / Grevlex). Coefficients use `num::BigRational` so no
//! intermediate during Buchberger overflows.

use std::cmp::Ordering;
use num::{BigInt, BigRational, One, Zero, ToPrimitive};
use maxima_core::{Expr, Operator, SymbolId};
use crate::poly::Poly;
use crate::coeff::Coeff;
use crate::gcd::poly_gcd;

pub type MCoeff = BigRational;

// ----------- Monomial -------------------------------------------------------

/// Exponent vector, one entry per variable in the parent `MPoly::vars`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Monomial(pub Vec<u32>);

impl Monomial {
    pub fn zero(nvars: usize) -> Self { Monomial(vec![0; nvars]) }

    pub fn total_degree(&self) -> u32 { self.0.iter().sum() }

    pub fn mul(&self, other: &Monomial) -> Monomial {
        debug_assert_eq!(self.0.len(), other.0.len());
        Monomial(self.0.iter().zip(other.0.iter()).map(|(a, b)| a + b).collect())
    }

    pub fn divides(&self, other: &Monomial) -> bool {
        debug_assert_eq!(self.0.len(), other.0.len());
        self.0.iter().zip(other.0.iter()).all(|(a, b)| a <= b)
    }

    /// `other / self`, only valid if `self.divides(other)`.
    pub fn div(other: &Monomial, divisor: &Monomial) -> Option<Monomial> {
        debug_assert_eq!(other.0.len(), divisor.0.len());
        if !divisor.divides(other) { return None; }
        Some(Monomial(other.0.iter().zip(divisor.0.iter()).map(|(a, b)| a - b).collect()))
    }

    pub fn lcm(&self, other: &Monomial) -> Monomial {
        debug_assert_eq!(self.0.len(), other.0.len());
        Monomial(self.0.iter().zip(other.0.iter()).map(|(a, b)| (*a).max(*b)).collect())
    }

    pub fn is_one(&self) -> bool { self.0.iter().all(|&e| e == 0) }
}

// ----------- Monomial order -------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonomialOrder {
    /// Lexicographic: compare exponent vectors entry-by-entry from index 0.
    Lex,
    /// Graded lexicographic: total degree first, then lex tie-break.
    Grlex,
    /// Graded reverse lex: total degree first, then for ties, the last
    /// nonzero entry of `a - b` decides — `a > b` iff that last differing
    /// entry has `a_i < b_i` (so x²y > xy² because y has smaller exponent).
    /// Fastest in practice for Buchberger; the standard default.
    Grevlex,
}

impl MonomialOrder {
    pub fn cmp(self, a: &Monomial, b: &Monomial) -> Ordering {
        debug_assert_eq!(a.0.len(), b.0.len());
        match self {
            MonomialOrder::Lex => a.0.cmp(&b.0),
            MonomialOrder::Grlex => {
                let da = a.total_degree();
                let db = b.total_degree();
                match da.cmp(&db) {
                    Ordering::Equal => a.0.cmp(&b.0),
                    other => other,
                }
            }
            MonomialOrder::Grevlex => {
                let da = a.total_degree();
                let db = b.total_degree();
                match da.cmp(&db) {
                    Ordering::Equal => {
                        // Last index where a and b differ; reverse comparison.
                        for i in (0..a.0.len()).rev() {
                            if a.0[i] != b.0[i] {
                                return b.0[i].cmp(&a.0[i]);
                            }
                        }
                        Ordering::Equal
                    }
                    other => other,
                }
            }
        }
    }
}

// ----------- MPoly ----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MPoly {
    pub vars: Vec<SymbolId>,
    pub order: MonomialOrder,
    /// Terms sorted *descending* under `order`; no zero coefficients.
    pub terms: Vec<(Monomial, MCoeff)>,
}

impl MPoly {
    pub fn zero(vars: Vec<SymbolId>, order: MonomialOrder) -> Self {
        MPoly { vars, order, terms: vec![] }
    }

    pub fn constant(vars: Vec<SymbolId>, order: MonomialOrder, c: MCoeff) -> Self {
        let nvars = vars.len();
        if c.is_zero() {
            MPoly::zero(vars, order)
        } else {
            MPoly { vars, order, terms: vec![(Monomial::zero(nvars), c)] }
        }
    }

    pub fn nvars(&self) -> usize { self.vars.len() }
    pub fn is_zero(&self) -> bool { self.terms.is_empty() }

    pub fn lt(&self) -> Option<(&Monomial, &MCoeff)> {
        self.terms.first().map(|(m, c)| (m, c))
    }
    pub fn lm(&self) -> Option<&Monomial> { self.terms.first().map(|(m, _)| m) }
    pub fn lc(&self) -> Option<&MCoeff> { self.terms.first().map(|(_, c)| c) }
    pub fn total_degree(&self) -> u32 {
        self.terms.iter().map(|(m, _)| m.total_degree()).max().unwrap_or(0)
    }

    fn canonicalize(&mut self) {
        self.terms.retain(|(_, c)| !c.is_zero());
        let ord = self.order;
        // Descending: ord.cmp(b, a) so larger comes first.
        self.terms.sort_by(|(a, _), (b, _)| ord.cmp(b, a));
    }

    /// Add `other`. Both must share vars and order.
    pub fn add(&self, other: &MPoly) -> MPoly {
        assert_eq!(self.vars, other.vars, "MPoly::add: variable mismatch");
        assert_eq!(self.order, other.order, "MPoly::add: order mismatch");
        let mut result = self.clone();
        for (m, c) in &other.terms {
            if let Some(pos) = result.terms.iter().position(|(rm, _)| rm == m) {
                result.terms[pos].1 = &result.terms[pos].1 + c;
            } else {
                result.terms.push((m.clone(), c.clone()));
            }
        }
        result.canonicalize();
        result
    }

    pub fn sub(&self, other: &MPoly) -> MPoly { self.add(&other.neg()) }

    pub fn neg(&self) -> MPoly {
        MPoly {
            vars: self.vars.clone(),
            order: self.order,
            terms: self.terms.iter().map(|(m, c)| (m.clone(), -c.clone())).collect(),
        }
    }

    pub fn scalar_mul(&self, c: &MCoeff) -> MPoly {
        if c.is_zero() {
            return MPoly::zero(self.vars.clone(), self.order);
        }
        MPoly {
            vars: self.vars.clone(),
            order: self.order,
            terms: self.terms.iter().map(|(m, k)| (m.clone(), k * c)).collect(),
        }
    }

    /// Multiply each term by `coef * mono`. Used by Buchberger and by full mul.
    pub fn monomial_mul(&self, coef: &MCoeff, mono: &Monomial) -> MPoly {
        if coef.is_zero() {
            return MPoly::zero(self.vars.clone(), self.order);
        }
        let terms = self.terms.iter()
            .map(|(m, c)| (m.mul(mono), c * coef))
            .collect();
        let mut result = MPoly { vars: self.vars.clone(), order: self.order, terms };
        // Ordering is preserved by monomial multiplication when nothing cancels,
        // but canonicalize anyway to drop zeros and to be a defensive no-op
        // when canonicalization is needed.
        result.canonicalize();
        result
    }

    pub fn mul(&self, other: &MPoly) -> MPoly {
        assert_eq!(self.vars, other.vars, "MPoly::mul: variable mismatch");
        assert_eq!(self.order, other.order, "MPoly::mul: order mismatch");
        let mut result = MPoly::zero(self.vars.clone(), self.order);
        for (m, c) in &other.terms {
            let term_poly = self.monomial_mul(c, m);
            result = result.add(&term_poly);
        }
        result
    }

    /// Exact division: returns Some(quotient) iff `divisor` divides `self`
    /// exactly (remainder zero), via repeated leading-term cancellation.
    /// Coefficients are over the field Q, so this is well-defined.
    pub fn exact_div(&self, divisor: &MPoly) -> Option<MPoly> {
        assert_eq!(self.vars, divisor.vars, "MPoly::exact_div: variable mismatch");
        let (dlm, dlc) = divisor.lt()?;
        let mut rem = self.clone();
        let mut quot = MPoly::zero(self.vars.clone(), self.order);
        loop {
            let (m, c) = match rem.lt() {
                None => break,
                Some((rlm, rlc)) => match Monomial::div(rlm, dlm) {
                    Some(m) => (m, rlc / dlc),
                    None => return None, // leading term not divisible
                },
            };
            let term = MPoly { vars: self.vars.clone(), order: self.order, terms: vec![(m.clone(), c.clone())] };
            quot = quot.add(&term);
            rem = rem.sub(&divisor.monomial_mul(&c, &m));
        }
        if rem.is_zero() { Some(quot) } else { None }
    }
}

// ----------- Expr ↔ MPoly --------------------------------------------------

fn coeff_from_expr(e: &Expr) -> Option<MCoeff> {
    match e {
        Expr::Integer(n) => Some(BigRational::from_integer(BigInt::from(*n))),
        Expr::Rational { num, den } => Some(BigRational::new(BigInt::from(*num), BigInt::from(*den))),
        Expr::BigInt(b) => Some(BigRational::from_integer((**b).clone())),
        _ => None,
    }
}

fn var_index(id: SymbolId, vars: &[SymbolId]) -> Option<usize> {
    vars.iter().position(|v| *v == id)
}

/// Convert an `Expr` to an `MPoly` over the given variables. Returns `None`
/// if the expression isn't expressible as a polynomial in those vars with
/// rational coefficients (e.g. contains a non-listed symbol, a function call,
/// a non-integer exponent, or a negative exponent).
pub fn expr_to_mpoly(e: &Expr, vars: &[SymbolId], order: MonomialOrder) -> Option<MPoly> {
    let one = MPoly::constant(vars.to_vec(), order, MCoeff::one());

    if let Some(c) = coeff_from_expr(e) {
        return Some(MPoly::constant(vars.to_vec(), order, c));
    }
    match e {
        Expr::Symbol(id) => {
            let i = var_index(*id, vars)?;
            let mut m = Monomial::zero(vars.len());
            m.0[i] = 1;
            Some(MPoly { vars: vars.to_vec(), order, terms: vec![(m, MCoeff::one())] })
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut acc = MPoly::zero(vars.to_vec(), order);
            for a in args {
                acc = acc.add(&expr_to_mpoly(a, vars, order)?);
            }
            Some(acc)
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut acc = one;
            for a in args {
                acc = acc.mul(&expr_to_mpoly(a, vars, order)?);
            }
            Some(acc)
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let n = match &args[1] {
                Expr::Integer(n) if *n >= 0 => *n as u32,
                _ => return None,
            };
            // Special-case base = a known variable for efficiency / clarity.
            if let Expr::Symbol(id) = &args[0] {
                if let Some(i) = var_index(*id, vars) {
                    let mut m = Monomial::zero(vars.len());
                    m.0[i] = n;
                    return Some(MPoly {
                        vars: vars.to_vec(),
                        order,
                        terms: vec![(m, MCoeff::one())],
                    });
                }
            }
            // General base: repeatedly multiply.
            let base = expr_to_mpoly(&args[0], vars, order)?;
            let mut acc = MPoly::constant(vars.to_vec(), order, MCoeff::one());
            for _ in 0..n { acc = acc.mul(&base); }
            Some(acc)
        }
        _ => None,
    }
}

fn bigint_to_expr(b: &BigInt) -> Expr {
    match b.to_i64() {
        Some(i) => Expr::int(i),
        None => Expr::BigInt(Box::new(b.clone())),
    }
}

fn coeff_to_expr(c: &MCoeff) -> Expr {
    let num = bigint_to_expr(c.numer());
    if c.denom().is_one() { num } else { Expr::div(num, bigint_to_expr(c.denom())) }
}

/// Convert an MPoly back to an Expr. Produces a sum of products in
/// descending order under the poly's `MonomialOrder`. The host simplifier
/// is expected to fold the result further if it cares about display order.
pub fn mpoly_to_expr(p: &MPoly) -> Expr {
    if p.is_zero() { return Expr::int(0); }
    let mut term_exprs = Vec::new();
    for (m, c) in &p.terms {
        let mut factors: Vec<Expr> = Vec::new();
        let coef_expr = coeff_to_expr(c);
        let coef_is_one = matches!(&coef_expr, Expr::Integer(1));
        let coef_is_neg_one = matches!(&coef_expr, Expr::Integer(-1));
        if !coef_is_one && !coef_is_neg_one {
            factors.push(coef_expr);
        } else if m.is_one() {
            // Plain constant.
            factors.push(if coef_is_neg_one { Expr::int(-1) } else { Expr::int(1) });
        }
        for (i, &e) in m.0.iter().enumerate() {
            if e == 0 { continue; }
            let sym = Expr::Symbol(p.vars[i]);
            if e == 1 { factors.push(sym); }
            else { factors.push(Expr::pow(sym, Expr::int(e as i64))); }
        }
        let term = if factors.len() == 1 {
            factors.pop().unwrap()
        } else {
            Expr::List { op: Operator::MTimes, simplified: false, args: factors }
        };
        let term = if coef_is_neg_one && !m.is_one() {
            Expr::neg(term)
        } else {
            term
        };
        term_exprs.push(term);
    }
    if term_exprs.len() == 1 {
        term_exprs.pop().unwrap()
    } else {
        Expr::List { op: Operator::MPlus, simplified: false, args: term_exprs }
    }
}

// ----------- Unit tests -----------------------------------------------------

// ----------- Multivariate GCD via Kronecker substitution -------------------

fn mcoeff_to_coeff(r: &MCoeff) -> Option<Coeff> {
    let n = r.numer().to_i64()?;
    let d = r.denom().to_i64()?;
    if d == 1 { Some(Coeff::Int(n)) } else { Some(Coeff::Rat(n, d)) }
}

fn coeff_to_mcoeff(c: &Coeff) -> MCoeff {
    match c {
        Coeff::Int(n) => BigRational::from(BigInt::from(*n)),
        Coeff::Rat(n, d) => BigRational::new(BigInt::from(*n), BigInt::from(*d)),
    }
}

fn max_var_exp(p: &MPoly) -> u32 {
    p.terms.iter().flat_map(|(m, _)| m.0.iter().copied()).max().unwrap_or(0)
}

/// Kronecker map: monomial ∏ x_i^{e_i} ↦ t^{Σ e_i·d^i}. None if a coefficient
/// doesn't fit i64 or a t-exponent overflows u32.
fn to_kronecker(p: &MPoly, d: u32, var: SymbolId) -> Option<Poly> {
    let mut terms: Vec<(u32, Coeff)> = Vec::new();
    for (m, c) in &p.terms {
        let mut texp: u64 = 0;
        let mut place: u64 = 1;
        for &e in &m.0 {
            texp = texp.checked_add((e as u64).checked_mul(place)?)?;
            place = place.checked_mul(d as u64)?;
        }
        if texp > u32::MAX as u64 { return None; }
        terms.push((texp as u32, mcoeff_to_coeff(c)?));
    }
    Some(Poly { var, terms })
}

/// Invert the Kronecker map for `nvars` variables in base `d`.
fn from_kronecker(p: &Poly, d: u32, nvars: usize, vars: Vec<SymbolId>, order: MonomialOrder) -> MPoly {
    let mut terms = Vec::new();
    for (texp, c) in &p.terms {
        let mut e = *texp;
        let mut exps = vec![0u32; nvars];
        for slot in exps.iter_mut() {
            *slot = e % d;
            e /= d;
        }
        terms.push((Monomial(exps), coeff_to_mcoeff(c)));
    }
    let mut result = MPoly { vars, order, terms };
    result.canonicalize();
    result
}

fn make_monic(p: &MPoly) -> MPoly {
    match p.lc() {
        Some(lc) if !lc.is_zero() => p.scalar_mul(&(MCoeff::one() / lc.clone())),
        _ => p.clone(),
    }
}

/// Multivariate GCD over Q via Kronecker substitution + univariate `poly_gcd`,
/// verified by exact division. Returns the monic gcd when it divides both
/// inputs (then it is provably the gcd, by a degree argument); otherwise the
/// constant 1 — a safe, possibly-incomplete answer, never a wrong one.
pub fn mpoly_gcd(a: &MPoly, b: &MPoly) -> MPoly {
    assert_eq!(a.vars, b.vars, "mpoly_gcd: variable mismatch");
    let one = MPoly::constant(a.vars.clone(), a.order, MCoeff::one());
    if a.is_zero() { return make_monic(b); }
    if b.is_zero() { return make_monic(a); }
    // gcd with a constant is a unit → 1
    if a.lm().map_or(true, |m| m.is_one()) || b.lm().map_or(true, |m| m.is_one()) {
        return one;
    }
    let d = 1 + max_var_exp(a).max(max_var_exp(b));
    let var = a.vars[0];
    let (Some(ka), Some(kb)) = (to_kronecker(a, d, var), to_kronecker(b, d, var)) else {
        return one;
    };
    let g_uni = poly_gcd(&ka, &kb);
    if g_uni.terms.is_empty() { return one; }
    let g = make_monic(&from_kronecker(&g_uni, d, a.nvars(), a.vars.clone(), a.order));
    if g.lm().map_or(true, |m| m.is_one()) { return one; }
    if a.exact_div(&g).is_some() && b.exact_div(&g).is_some() {
        g
    } else {
        one
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::intern;

    fn vars2() -> Vec<SymbolId> { vec![intern("x"), intern("y")] }

    fn mono(es: &[u32]) -> Monomial { Monomial(es.to_vec()) }

    // ---- monomial ops ----

    #[test] fn mono_mul_div_lcm() {
        let a = mono(&[2, 1]);
        let b = mono(&[1, 3]);
        assert_eq!(a.mul(&b), mono(&[3, 4]));
        assert_eq!(a.lcm(&b), mono(&[2, 3]));
        let ab = a.mul(&b);
        assert_eq!(Monomial::div(&ab, &a).as_ref(), Some(&b));
        assert!(a.divides(&ab));
        assert!(!a.divides(&b));
    }

    // ---- monomial orders ----

    #[test] fn lex_orders_first_coord_first() {
        let ord = MonomialOrder::Lex;
        // x^2 > x*y under lex because first coord (2 vs 1) decides.
        assert_eq!(ord.cmp(&mono(&[2, 0]), &mono(&[1, 5])), Ordering::Greater);
        assert_eq!(ord.cmp(&mono(&[1, 5]), &mono(&[2, 0])), Ordering::Less);
    }

    #[test] fn grlex_uses_total_degree_first() {
        let ord = MonomialOrder::Grlex;
        // x*y > x^2 if total degrees agree it falls back to lex, but here
        // both have degree 2 and lex says x^2 > x*y. So x^2 > x*y.
        assert_eq!(ord.cmp(&mono(&[2, 0]), &mono(&[1, 1])), Ordering::Greater);
        // Degree decides when unequal.
        assert_eq!(ord.cmp(&mono(&[1, 1]), &mono(&[5, 0])), Ordering::Less);
    }

    #[test] fn grevlex_x_squared_beats_x_y_squared() {
        // Standard textbook example: under grevlex with vars=[x,y],
        // x^2*y > x*y^2  (both degree 3, last-coord-smaller wins).
        let ord = MonomialOrder::Grevlex;
        assert_eq!(ord.cmp(&mono(&[2, 1]), &mono(&[1, 2])), Ordering::Greater);
    }

    // ---- MPoly arithmetic ----

    #[test] fn mpoly_add_collects_like_terms() {
        let v = vars2();
        // (x + y) + (x - y) = 2*x
        let xpy = expr_to_mpoly(
            &Expr::add(Expr::sym("x"), Expr::sym("y")),
            &v, MonomialOrder::Grlex).unwrap();
        let xmy = expr_to_mpoly(
            &Expr::sub(Expr::sym("x"), Expr::sym("y")),
            &v, MonomialOrder::Grlex).unwrap();
        let sum = xpy.add(&xmy);
        assert_eq!(sum.terms.len(), 1);
        assert_eq!(sum.terms[0].0, mono(&[1, 0]));
        assert_eq!(sum.terms[0].1, BigRational::from_integer(2.into()));
    }

    #[test] fn mpoly_mul_distributes() {
        let v = vars2();
        // (x + y) * (x - y) = x^2 - y^2
        let a = expr_to_mpoly(
            &Expr::add(Expr::sym("x"), Expr::sym("y")), &v, MonomialOrder::Grlex).unwrap();
        let b = expr_to_mpoly(
            &Expr::sub(Expr::sym("x"), Expr::sym("y")), &v, MonomialOrder::Grlex).unwrap();
        let prod = a.mul(&b);
        // Expect two terms: x^2 with coeff +1 and y^2 with coeff -1.
        assert_eq!(prod.terms.len(), 2);
        assert!(prod.terms.iter().any(|(m, c)|
            *m == mono(&[2, 0]) && *c == BigRational::from_integer(1.into())));
        assert!(prod.terms.iter().any(|(m, c)|
            *m == mono(&[0, 2]) && *c == BigRational::from_integer((-1).into())));
    }

    #[test] fn mpoly_leading_term_under_each_order() {
        let v = vars2();
        // x^2*y + x*y^2: leading term differs by order.
        let e = Expr::add(
            Expr::mul(Expr::pow(Expr::sym("x"), Expr::int(2)), Expr::sym("y")),
            Expr::mul(Expr::sym("x"), Expr::pow(Expr::sym("y"), Expr::int(2))),
        );
        let p_lex = expr_to_mpoly(&e, &v, MonomialOrder::Lex).unwrap();
        let p_glx = expr_to_mpoly(&e, &v, MonomialOrder::Grlex).unwrap();
        let p_grv = expr_to_mpoly(&e, &v, MonomialOrder::Grevlex).unwrap();
        // Under all three, the lead is x^2*y here (lex: 2 > 1 in first coord;
        // grlex: equal degree, lex tie x^2*y > x*y^2; grevlex: equal degree,
        // last-coord-smaller x^2*y has y^1 vs x*y^2 y^2, x^2*y wins).
        assert_eq!(p_lex.lm().unwrap(), &mono(&[2, 1]));
        assert_eq!(p_glx.lm().unwrap(), &mono(&[2, 1]));
        assert_eq!(p_grv.lm().unwrap(), &mono(&[2, 1]));
    }

    // ---- Expr round trip ----

    #[test] fn round_trip_polynomial() {
        let v = vars2();
        // 3*x^2 + 2*x*y - y^2 + 1
        let e = Expr::List {
            op: Operator::MPlus, simplified: false,
            args: vec![
                Expr::mul(Expr::int(3), Expr::pow(Expr::sym("x"), Expr::int(2))),
                Expr::mul(Expr::int(2), Expr::mul(Expr::sym("x"), Expr::sym("y"))),
                Expr::neg(Expr::pow(Expr::sym("y"), Expr::int(2))),
                Expr::int(1),
            ],
        };
        let p = expr_to_mpoly(&e, &v, MonomialOrder::Grlex).unwrap();
        // Should have four terms.
        assert_eq!(p.terms.len(), 4);
        // Round trip and re-parse: structure may differ but back into MPoly
        // it must equal `p`.
        let e2 = mpoly_to_expr(&p);
        let p2 = expr_to_mpoly(&e2, &v, MonomialOrder::Grlex).unwrap();
        assert_eq!(p, p2);
    }

    #[test] fn unknown_symbol_yields_none() {
        // z is not in the variable list — should be reported as not-a-poly.
        let v = vars2();
        let bad = Expr::add(Expr::sym("x"), Expr::sym("z"));
        assert!(expr_to_mpoly(&bad, &v, MonomialOrder::Lex).is_none());
    }

    #[test] fn float_coeff_yields_none() {
        // Only exact rationals supported.
        let v = vars2();
        let bad = Expr::mul(Expr::Float(0.5), Expr::sym("x"));
        assert!(expr_to_mpoly(&bad, &v, MonomialOrder::Lex).is_none());
    }

    // ---- exact division & multivariate GCD ----

    fn mp(e: &Expr) -> MPoly {
        expr_to_mpoly(e, &vars2(), MonomialOrder::Grevlex).unwrap()
    }
    fn x() -> Expr { Expr::sym("x") }
    fn y() -> Expr { Expr::sym("y") }
    fn sq(e: Expr) -> Expr { Expr::pow(e, Expr::int(2)) }

    #[test] fn exact_div_divides_and_rejects() {
        // (x²-y²) / (x-y) = x+y
        let a = mp(&Expr::sub(sq(x()), sq(y())));
        let d = mp(&Expr::sub(x(), y()));
        let q = a.exact_div(&d).expect("x-y divides x^2-y^2");
        assert_eq!(q, mp(&Expr::add(x(), y())));
        // (x²-y²) / (x+1) does not divide exactly
        let nd = mp(&Expr::add(x(), Expr::int(1)));
        assert!(a.exact_div(&nd).is_none());
    }

    #[test] fn gcd_difference_of_squares() {
        let a = mp(&Expr::sub(sq(x()), sq(y())));            // x²-y²
        let b = mp(&Expr::sub(x(), y()));                    // x-y
        let g = mpoly_gcd(&a, &b);
        assert!(a.exact_div(&g).is_some() && b.exact_div(&g).is_some());
        assert_eq!(g, mp(&Expr::sub(x(), y())));             // gcd = x-y
    }

    #[test] fn gcd_common_linear_factor() {
        // gcd(x²-y², x²+2xy+y²) = x+y
        let a = mp(&Expr::sub(sq(x()), sq(y())));
        let b = mp(&Expr::add(Expr::add(sq(x()), Expr::mul(Expr::int(2), Expr::mul(x(), y()))), sq(y())));
        let g = mpoly_gcd(&a, &b);
        assert!(a.exact_div(&g).is_some() && b.exact_div(&g).is_some());
        assert_eq!(g, mp(&Expr::add(x(), y())));             // gcd = x+y
    }

    #[test] fn gcd_coprime_is_one() {
        let a = mp(&Expr::add(x(), y()));
        let b = mp(&Expr::sub(x(), y()));
        let g = mpoly_gcd(&a, &b);
        assert_eq!(g.total_degree(), 0);                     // coprime → constant
    }
}
