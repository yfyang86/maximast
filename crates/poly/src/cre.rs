use maxima_core::SymbolId;
use crate::coeff::Coeff;
use crate::poly::Poly;
use crate::gcd::poly_gcd;
use crate::traits::{Ring, DifferentialRing};

/// Canonical Rational Expression: p(x)/q(x) in lowest terms.
/// This is the core data structure for the Risch algorithm.
#[derive(Debug, Clone, PartialEq)]
pub struct CRE {
    pub num: Poly,
    pub den: Poly,
    pub var: SymbolId,
}

impl CRE {
    /// Create a new CRE, normalizing to lowest terms.
    pub fn new(num: Poly, den: Poly) -> Self {
        let var = num.var;
        assert_eq!(num.var, den.var);

        if den.is_zero() {
            panic!("CRE: division by zero");
        }

        // Cancel common factors
        let g = poly_gcd(&num, &den);
        let (num, den) = if g.is_constant() && g.leading_coeff().is_one() {
            (num, den)
        } else {
            let n = num.exact_div(&g).unwrap_or(num);
            let d = den.exact_div(&g).unwrap_or(den);
            (n, d)
        };

        // Normalize: make leading coefficient of denominator positive
        let lc = den.leading_coeff();
        let (num, den) = if matches!(&lc, Coeff::Int(n) if *n < 0) {
            (num.neg(), den.neg())
        } else {
            (num, den)
        };

        CRE { num, den, var }
    }

    pub fn from_poly(p: Poly) -> Self {
        let var = p.var;
        CRE { num: p, den: Poly::constant(var, Coeff::one()), var }
    }

    pub fn from_int(n: i64, var: SymbolId) -> Self {
        CRE {
            num: Poly::constant(var, Coeff::Int(n)),
            den: Poly::constant(var, Coeff::one()),
            var,
        }
    }

    pub fn zero(var: SymbolId) -> Self {
        CRE {
            num: Poly::zero(var),
            den: Poly::constant(var, Coeff::one()),
            var,
        }
    }

    pub fn one(var: SymbolId) -> Self {
        Self::from_int(1, var)
    }

    pub fn is_zero(&self) -> bool { self.num.is_zero() }
    pub fn is_constant(&self) -> bool { self.num.is_constant() && self.den.is_constant() }
    pub fn is_polynomial(&self) -> bool { self.den.is_constant() }

    /// Addition: a/b + c/d = (a*d + c*b) / (b*d)
    pub fn add(&self, other: &CRE) -> CRE {
        let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
        let den = self.den.mul(&other.den);
        CRE::new(num, den)
    }

    /// Subtraction
    pub fn sub(&self, other: &CRE) -> CRE {
        let num = self.num.mul(&other.den).sub(&other.num.mul(&self.den));
        let den = self.den.mul(&other.den);
        CRE::new(num, den)
    }

    /// Multiplication: (a/b) * (c/d) = (a*c) / (b*d)
    pub fn mul(&self, other: &CRE) -> CRE {
        let num = self.num.mul(&other.num);
        let den = self.den.mul(&other.den);
        CRE::new(num, den)
    }

    /// Division: (a/b) / (c/d) = (a*d) / (b*c)
    pub fn div(&self, other: &CRE) -> Option<CRE> {
        if other.is_zero() { return None; }
        let num = self.num.mul(&other.den);
        let den = self.den.mul(&other.num);
        Some(CRE::new(num, den))
    }

    /// Negation
    pub fn neg(&self) -> CRE {
        CRE { num: self.num.neg(), den: self.den.clone(), var: self.var }
    }

    /// Derivative using quotient rule: (n/d)' = (n'd - nd') / d²
    pub fn derivative(&self) -> CRE {
        let n_prime = self.num.derivative();
        let d_prime = self.den.derivative();
        let num = n_prime.mul(&self.den).sub(&self.num.mul(&d_prime));
        let den = self.den.mul(&self.den);
        CRE::new(num, den)
    }

    /// Evaluate at a rational point
    pub fn eval_at(&self, x: &Coeff) -> Option<Coeff> {
        let n = self.num.eval_at(x);
        let d = self.den.eval_at(x);
        if d.is_zero() { return None; }
        n.div(&d)
    }

    /// Degree of numerator minus degree of denominator
    pub fn degree_diff(&self) -> i32 {
        let nd = self.num.degree().unwrap_or(0) as i32;
        let dd = self.den.degree().unwrap_or(0) as i32;
        nd - dd
    }
}

// CRE implements derivative but not the full Ring trait yet
// (Ring requires Clone+PartialEq which CRE has, but the operations
// need careful normalization that doesn't fit the simple Ring interface)
impl CRE {
    pub fn deriv_cre(&self) -> Self {
        self.derivative()
    }
}

impl std::fmt::Display for CRE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.den.is_constant() && self.den.leading_coeff().is_one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "({})/({})", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::intern;

    fn x() -> SymbolId { intern("x") }
    fn p(terms: &[(u32, i64)]) -> Poly {
        Poly { var: x(), terms: terms.iter().map(|(e, c)| (*e, Coeff::Int(*c))).collect() }
    }

    #[test]
    fn cre_normalize() {
        // (2x+2)/(x+1) should reduce to 2/1
        let num = p(&[(1, 2), (0, 2)]); // 2x+2
        let den = p(&[(1, 1), (0, 1)]); // x+1
        let cre = CRE::new(num, den);
        assert!(cre.is_constant() || cre.is_polynomial());
    }

    #[test]
    fn cre_add() {
        // 1/x + 1/(x+1)
        let a = CRE::new(p(&[(0, 1)]), p(&[(1, 1)])); // 1/x
        let b = CRE::new(p(&[(0, 1)]), p(&[(1, 1), (0, 1)])); // 1/(x+1)
        let sum = a.add(&b);
        // Should be (2x+1)/(x²+x)
        assert_eq!(sum.num.degree(), Some(1));
        assert_eq!(sum.den.degree(), Some(2));
    }

    #[test]
    fn cre_derivative() {
        // d/dx (1/x) = -1/x²
        let f = CRE::new(p(&[(0, 1)]), p(&[(1, 1)]));
        let df = f.derivative();
        // num should be -1, den should be x²
        assert_eq!(df.num.degree(), Some(0));
        assert_eq!(df.den.degree(), Some(2));
    }

    #[test]
    fn cre_mul() {
        // (x+1)/x * x/(x-1) = (x+1)/(x-1)
        let a = CRE::new(p(&[(1, 1), (0, 1)]), p(&[(1, 1)]));
        let b = CRE::new(p(&[(1, 1)]), p(&[(1, 1), (0, -1)]));
        let prod = a.mul(&b);
        assert_eq!(prod.num, p(&[(1, 1), (0, 1)])); // x+1
        assert_eq!(prod.den, p(&[(1, 1), (0, -1)])); // x-1
    }

    #[test]
    fn cre_zero() {
        let z = CRE::zero(x());
        assert!(z.is_zero());
    }

    #[test]
    fn cre_eval() {
        // (x+1)/(x-1) at x=3 → 4/2 = 2
        let f = CRE::new(p(&[(1, 1), (0, 1)]), p(&[(1, 1), (0, -1)]));
        assert_eq!(f.eval_at(&Coeff::Int(3)), Some(Coeff::Int(2)));
    }

    #[test]
    fn cre_display() {
        let f = CRE::new(p(&[(1, 1), (0, 1)]), p(&[(1, 1), (0, -1)]));
        let s = f.to_string();
        assert!(s.contains("/"), "got: {}", s);
    }
}
