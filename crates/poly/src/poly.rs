use std::fmt;
use maxima_core::SymbolId;
use crate::coeff::Coeff;

/// Sparse univariate polynomial: list of (exponent, coefficient) pairs,
/// sorted by descending exponent. Coefficients are in Q (rationals).
///
/// For multivariate: coefficients can be wrapped polynomials in the next variable.
/// This recursive structure is used when converting from/to Expr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poly {
    pub var: SymbolId,
    /// Terms sorted by descending exponent. No zero coefficients.
    pub terms: Vec<(u32, Coeff)>,
}

impl Poly {
    pub fn zero(var: SymbolId) -> Self {
        Poly { var, terms: vec![] }
    }

    pub fn constant(var: SymbolId, c: Coeff) -> Self {
        if c.is_zero() {
            Poly::zero(var)
        } else {
            Poly { var, terms: vec![(0, c)] }
        }
    }

    pub fn monomial(var: SymbolId, exp: u32, coeff: Coeff) -> Self {
        if coeff.is_zero() {
            Poly::zero(var)
        } else {
            Poly { var, terms: vec![(exp, coeff)] }
        }
    }

    /// x (the variable itself, coefficient 1)
    pub fn var_poly(var: SymbolId) -> Self {
        Poly { var, terms: vec![(1, Coeff::one())] }
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn is_constant(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].0 == 0)
    }

    pub fn degree(&self) -> Option<u32> {
        self.terms.first().map(|(e, _)| *e)
    }

    pub fn leading_coeff(&self) -> Coeff {
        self.terms.first().map(|(_, c)| c.clone()).unwrap_or(Coeff::zero())
    }

    pub fn constant_term(&self) -> Coeff {
        self.terms.iter()
            .find(|(e, _)| *e == 0)
            .map(|(_, c)| c.clone())
            .unwrap_or(Coeff::zero())
    }

    pub fn neg(&self) -> Self {
        Poly {
            var: self.var,
            terms: self.terms.iter().map(|(e, c)| (*e, c.neg())).collect(),
        }
    }

    pub fn scale(&self, c: &Coeff) -> Self {
        if c.is_zero() {
            return Poly::zero(self.var);
        }
        let terms: Vec<(u32, Coeff)> = self.terms.iter()
            .map(|(e, tc)| (*e, tc.mul(c)))
            .filter(|(_, tc)| !tc.is_zero())
            .collect();
        Poly { var: self.var, terms }
    }

    pub fn add(&self, other: &Poly) -> Poly {
        assert_eq!(self.var, other.var);
        let mut terms: Vec<(u32, Coeff)> = Vec::new();
        let (mut i, mut j) = (0, 0);

        while i < self.terms.len() && j < other.terms.len() {
            let (e1, c1) = &self.terms[i];
            let (e2, c2) = &other.terms[j];
            match e1.cmp(e2) {
                std::cmp::Ordering::Greater => {
                    terms.push((*e1, c1.clone()));
                    i += 1;
                }
                std::cmp::Ordering::Less => {
                    terms.push((*e2, c2.clone()));
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    let sum = c1.add(c2);
                    if !sum.is_zero() {
                        terms.push((*e1, sum));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
        while i < self.terms.len() {
            terms.push(self.terms[i].clone());
            i += 1;
        }
        while j < other.terms.len() {
            terms.push(other.terms[j].clone());
            j += 1;
        }
        Poly { var: self.var, terms }
    }

    pub fn sub(&self, other: &Poly) -> Poly {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Poly) -> Poly {
        assert_eq!(self.var, other.var);
        if self.is_zero() || other.is_zero() {
            return Poly::zero(self.var);
        }

        let mut result = Poly::zero(self.var);
        for (e1, c1) in &self.terms {
            for (e2, c2) in &other.terms {
                let term = Poly {
                    var: self.var,
                    terms: vec![(e1 + e2, c1.mul(c2))],
                };
                result = result.add(&term);
            }
        }
        result
    }

    /// Polynomial long division. Returns (quotient, remainder).
    pub fn divmod(&self, divisor: &Poly) -> Option<(Poly, Poly)> {
        assert_eq!(self.var, divisor.var);
        if divisor.is_zero() {
            return None;
        }

        let mut quotient = Poly::zero(self.var);
        let mut remainder = self.clone();
        let lc_divisor = divisor.leading_coeff();
        let deg_divisor = divisor.degree().unwrap();

        while !remainder.is_zero() {
            let deg_rem = match remainder.degree() {
                Some(d) => d,
                None => break,
            };
            if deg_rem < deg_divisor {
                break;
            }
            let lc_rem = remainder.leading_coeff();
            let coeff = match lc_rem.div(&lc_divisor) {
                Some(c) => c,
                None => break,
            };
            let deg_diff = deg_rem - deg_divisor;

            let term = Poly::monomial(self.var, deg_diff, coeff);
            quotient = quotient.add(&term);
            let sub = term.mul(divisor);
            remainder = remainder.sub(&sub);
        }
        Some((quotient, remainder))
    }

    /// Exact division (returns None if there's a remainder).
    pub fn exact_div(&self, divisor: &Poly) -> Option<Poly> {
        let (q, r) = self.divmod(divisor)?;
        if r.is_zero() { Some(q) } else { None }
    }

    /// Content: GCD of all coefficients.
    pub fn content(&self) -> Coeff {
        if self.terms.is_empty() {
            return Coeff::zero();
        }
        // For integer coefficients, compute GCD
        let mut result: Option<i64> = None;
        for (_, c) in &self.terms {
            if let Coeff::Int(n) = c {
                result = Some(match result {
                    Some(g) => gcd_i64(g.abs(), n.abs()),
                    None => n.abs(),
                });
            } else {
                return Coeff::one();
            }
        }
        Coeff::Int(result.unwrap_or(1))
    }

    /// Primitive part: self / content(self).
    pub fn primitive_part(&self) -> Poly {
        let c = self.content();
        if c.is_one() || c.is_zero() {
            return self.clone();
        }
        self.scale(&c.div(&Coeff::Int(1)).unwrap_or(Coeff::one()))
    }

    /// Evaluate at a point.
    pub fn eval_at(&self, x: &Coeff) -> Coeff {
        let mut result = Coeff::zero();
        for (e, c) in &self.terms {
            let mut x_pow = Coeff::one();
            for _ in 0..*e {
                x_pow = x_pow.mul(x);
            }
            result = result.add(&c.mul(&x_pow));
        }
        result
    }

    /// Formal derivative.
    pub fn derivative(&self) -> Poly {
        let terms: Vec<(u32, Coeff)> = self.terms.iter()
            .filter(|(e, _)| *e > 0)
            .map(|(e, c)| (e - 1, c.mul(&Coeff::Int(*e as i64))))
            .filter(|(_, c)| !c.is_zero())
            .collect();
        Poly { var: self.var, terms }
    }
}

fn gcd_i64(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd_i64(b, a % b) }
}

impl fmt::Display for Poly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }
        let var_name = maxima_core::resolve(self.var);
        for (i, (e, c)) in self.terms.iter().enumerate() {
            if i > 0 {
                if matches!(c, Coeff::Int(n) if *n < 0) || matches!(c, Coeff::Rat(n, _) if *n < 0) {
                    write!(f, " - ")?;
                    let pos = c.abs();
                    if *e == 0 {
                        write!(f, "{}", pos)?;
                    } else if pos.is_one() {
                        if *e == 1 { write!(f, "{}", var_name)?; }
                        else { write!(f, "{}^{}", var_name, e)?; }
                    } else {
                        if *e == 1 { write!(f, "{}*{}", pos, var_name)?; }
                        else { write!(f, "{}*{}^{}", pos, var_name, e)?; }
                    }
                    continue;
                }
                write!(f, " + ")?;
            }
            if *e == 0 {
                write!(f, "{}", c)?;
            } else if c.is_one() {
                if *e == 1 { write!(f, "{}", var_name)?; }
                else { write!(f, "{}^{}", var_name, e)?; }
            } else if *c == Coeff::Int(-1) && i == 0 {
                if *e == 1 { write!(f, "-{}", var_name)?; }
                else { write!(f, "-{}^{}", var_name, e)?; }
            } else {
                if *e == 1 { write!(f, "{}*{}", c, var_name)?; }
                else { write!(f, "{}*{}^{}", c, var_name, e)?; }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::intern;

    fn x() -> SymbolId { intern("x") }

    fn p(terms: &[(u32, i64)]) -> Poly {
        Poly {
            var: x(),
            terms: terms.iter().map(|(e, c)| (*e, Coeff::Int(*c))).collect(),
        }
    }

    #[test]
    fn poly_add() {
        let a = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1
        let b = p(&[(2, 1), (1, -2), (0, 1)]); // x^2-2x+1
        let sum = a.add(&b);
        assert_eq!(sum, p(&[(2, 2), (0, 2)])); // 2x^2+2
    }

    #[test]
    fn poly_sub() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let b = p(&[(2, 1), (0, -1)]); // x^2-1
        let diff = a.sub(&b);
        assert_eq!(diff, p(&[(0, 2)])); // 2
    }

    #[test]
    fn poly_mul() {
        let a = p(&[(1, 1), (0, 1)]); // x+1
        let b = p(&[(1, 1), (0, -1)]); // x-1
        let prod = a.mul(&b);
        assert_eq!(prod, p(&[(2, 1), (0, -1)])); // x^2-1
    }

    #[test]
    fn poly_mul_triple() {
        let a = p(&[(1, 1), (0, 1)]); // x+1
        let sq = a.mul(&a);
        assert_eq!(sq, p(&[(2, 1), (1, 2), (0, 1)])); // x^2+2x+1
    }

    #[test]
    fn poly_divmod() {
        let a = p(&[(3, 1), (0, -1)]); // x^3-1
        let b = p(&[(1, 1), (0, -1)]); // x-1
        let (q, r) = a.divmod(&b).unwrap();
        assert_eq!(q, p(&[(2, 1), (1, 1), (0, 1)])); // x^2+x+1
        assert!(r.is_zero());
    }

    #[test]
    fn poly_divmod_remainder() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let b = p(&[(1, 1), (0, -1)]); // x-1
        let (q, r) = a.divmod(&b).unwrap();
        assert_eq!(q, p(&[(1, 1), (0, 1)])); // x+1
        assert_eq!(r, p(&[(0, 2)])); // 2
    }

    #[test]
    fn poly_derivative() {
        let a = p(&[(3, 1), (2, 3), (1, 2), (0, 5)]); // x^3+3x^2+2x+5
        let d = a.derivative();
        assert_eq!(d, p(&[(2, 3), (1, 6), (0, 2)])); // 3x^2+6x+2
    }

    #[test]
    fn poly_eval() {
        let a = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1
        assert_eq!(a.eval_at(&Coeff::Int(3)), Coeff::Int(16)); // 9+6+1
    }

    #[test]
    fn poly_content() {
        let a = p(&[(2, 6), (1, 4), (0, 2)]); // 6x^2+4x+2
        assert_eq!(a.content(), Coeff::Int(2));
    }

    #[test]
    fn poly_display() {
        let a = p(&[(2, 1), (1, 2), (0, 1)]);
        assert_eq!(a.to_string(), "x^2 + 2*x + 1");
    }

    #[test]
    fn poly_display_negative() {
        let a = p(&[(2, 1), (1, -2), (0, 1)]);
        assert_eq!(a.to_string(), "x^2 - 2*x + 1");
    }

    #[test]
    fn poly_zero_ops() {
        let z = Poly::zero(x());
        let a = p(&[(1, 1)]);
        assert_eq!(z.add(&a), a);
        assert_eq!(a.add(&z), a);
        assert!(z.mul(&a).is_zero());
    }

    #[test]
    fn poly_exact_div() {
        let a = p(&[(2, 1), (0, -1)]); // x^2-1
        let b = p(&[(1, 1), (0, 1)]); // x+1
        let q = a.exact_div(&b).unwrap();
        assert_eq!(q, p(&[(1, 1), (0, -1)])); // x-1
    }

    #[test]
    fn poly_exact_div_fails() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let b = p(&[(1, 1), (0, 1)]); // x+1
        assert!(a.exact_div(&b).is_none()); // not divisible
    }

    // --- Comprehensive polynomial tests ---

    #[test]
    fn poly_degree() {
        assert_eq!(p(&[(3, 1), (0, 1)]).degree(), Some(3));
        assert_eq!(p(&[(0, 5)]).degree(), Some(0));
        assert_eq!(Poly::zero(x()).degree(), None);
    }

    #[test]
    fn poly_leading_coeff() {
        assert_eq!(p(&[(3, 5), (1, 2)]).leading_coeff(), Coeff::Int(5));
        assert_eq!(Poly::zero(x()).leading_coeff(), Coeff::zero());
    }

    #[test]
    fn poly_constant_term() {
        assert_eq!(p(&[(2, 3), (0, 7)]).constant_term(), Coeff::Int(7));
        assert_eq!(p(&[(2, 3), (1, 1)]).constant_term(), Coeff::zero());
    }

    #[test]
    fn poly_is_constant() {
        assert!(p(&[(0, 5)]).is_constant());
        assert!(Poly::zero(x()).is_constant());
        assert!(!p(&[(1, 1)]).is_constant());
    }

    #[test]
    fn poly_neg() {
        let a = p(&[(2, 3), (0, -1)]);
        assert_eq!(a.neg(), p(&[(2, -3), (0, 1)]));
        assert_eq!(a.neg().neg(), a);
    }

    #[test]
    fn poly_scale() {
        let a = p(&[(2, 3), (1, 6), (0, 9)]);
        assert_eq!(a.scale(&Coeff::Int(2)), p(&[(2, 6), (1, 12), (0, 18)]));
        assert!(a.scale(&Coeff::zero()).is_zero());
    }

    #[test]
    fn poly_add_cancel() {
        let a = p(&[(2, 1), (1, 2), (0, 1)]);
        let b = a.neg();
        assert!(a.add(&b).is_zero());
    }

    #[test]
    fn poly_mul_monomial() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let m = Poly::monomial(x(), 3, Coeff::Int(2)); // 2*x^3
        let r = a.mul(&m);
        assert_eq!(r, p(&[(5, 2), (3, 2)])); // 2*x^5+2*x^3
    }

    #[test]
    fn poly_derivative_constant() {
        let a = p(&[(0, 42)]);
        assert!(a.derivative().is_zero());
    }

    #[test]
    fn poly_derivative_linear() {
        let a = p(&[(1, 5), (0, 3)]); // 5x+3
        assert_eq!(a.derivative(), p(&[(0, 5)])); // 5
    }

    #[test]
    fn poly_eval_zero() {
        let a = p(&[(3, 1), (1, -1)]); // x^3-x
        assert_eq!(a.eval_at(&Coeff::zero()), Coeff::zero());
    }

    #[test]
    fn poly_eval_one() {
        let a = p(&[(3, 1), (1, -1)]); // x^3-x
        assert_eq!(a.eval_at(&Coeff::one()), Coeff::zero());
    }

    #[test]
    fn poly_mul_associative() {
        let a = p(&[(1, 1), (0, 1)]); // x+1
        let b = p(&[(1, 1), (0, -1)]); // x-1
        let c = p(&[(1, 1), (0, 2)]); // x+2
        assert_eq!(a.mul(&b).mul(&c), a.mul(&b.mul(&c)));
    }

    #[test]
    fn poly_mul_commutative() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let b = p(&[(1, 3), (0, -2)]); // 3x-2
        assert_eq!(a.mul(&b), b.mul(&a));
    }

    #[test]
    fn poly_add_commutative() {
        let a = p(&[(2, 1), (0, 3)]);
        let b = p(&[(1, 2), (0, -1)]);
        assert_eq!(a.add(&b), b.add(&a));
    }

    #[test]
    fn poly_divmod_by_one() {
        let a = p(&[(3, 1), (1, 2), (0, -3)]); // x^3+2x-3
        let one = p(&[(0, 1)]); // 1
        let (q, r) = a.divmod(&one).unwrap();
        assert_eq!(q, a);
        assert!(r.is_zero());
    }

    #[test]
    fn poly_divmod_by_self() {
        let a = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1
        let (q, r) = a.divmod(&a).unwrap();
        assert_eq!(q, p(&[(0, 1)])); // 1
        assert!(r.is_zero());
    }

    #[test]
    fn poly_divmod_zero_dividend() {
        let z = Poly::zero(x());
        let b = p(&[(1, 1), (0, 1)]);
        let (q, r) = z.divmod(&b).unwrap();
        assert!(q.is_zero());
        assert!(r.is_zero());
    }

    #[test]
    fn poly_divmod_by_zero() {
        let a = p(&[(1, 1)]);
        assert!(a.divmod(&Poly::zero(x())).is_none());
    }

    #[test]
    fn poly_content_coprime() {
        let a = p(&[(2, 3), (1, 5), (0, 7)]); // 3x^2+5x+7
        assert_eq!(a.content(), Coeff::Int(1));
    }

    #[test]
    fn poly_display_constant() {
        assert_eq!(p(&[(0, 42)]).to_string(), "42");
    }

    #[test]
    fn poly_display_linear() {
        assert_eq!(p(&[(1, 1), (0, 3)]).to_string(), "x + 3");
    }

    #[test]
    fn poly_display_zero() {
        assert_eq!(Poly::zero(x()).to_string(), "0");
    }

    #[test]
    fn poly_display_negative_leading() {
        assert_eq!(p(&[(2, -1), (0, 1)]).to_string(), "-x^2 + 1");
    }

    #[test]
    fn poly_high_degree_mul() {
        // (x+1)^4 = x^4+4x^3+6x^2+4x+1
        let a = p(&[(1, 1), (0, 1)]);
        let a2 = a.mul(&a);
        let a4 = a2.mul(&a2);
        assert_eq!(a4, p(&[(4, 1), (3, 4), (2, 6), (1, 4), (0, 1)]));
    }

    #[test]
    fn poly_var_poly() {
        let v = Poly::var_poly(x());
        assert_eq!(v.degree(), Some(1));
        assert_eq!(v.leading_coeff(), Coeff::one());
        assert_eq!(v.to_string(), "x");
    }

    #[test]
    fn poly_monomial() {
        let m = Poly::monomial(x(), 5, Coeff::Int(3));
        assert_eq!(m.degree(), Some(5));
        assert_eq!(m.to_string(), "3*x^5");
    }
}
