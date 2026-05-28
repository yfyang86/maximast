use std::fmt;

/// Polynomial coefficient: integer, rational, or nested polynomial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coeff {
    Int(i64),
    Rat(i64, i64),
}

impl Coeff {
    pub fn zero() -> Self { Coeff::Int(0) }
    pub fn one() -> Self { Coeff::Int(1) }

    pub fn is_zero(&self) -> bool {
        match self {
            Coeff::Int(0) => true,
            Coeff::Rat(0, _) => true,
            _ => false,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Coeff::Int(1) => true,
            Coeff::Rat(n, d) => *n == *d,
            _ => false,
        }
    }

    pub fn neg(&self) -> Self {
        match self {
            Coeff::Int(n) => Coeff::Int(-n),
            Coeff::Rat(n, d) => Coeff::Rat(-n, *d),
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (Coeff::Int(a), Coeff::Int(b)) => Coeff::Int(a + b),
            (Coeff::Int(a), Coeff::Rat(n, d)) | (Coeff::Rat(n, d), Coeff::Int(a)) => {
                Coeff::Rat(a * d + n, *d).reduce()
            }
            (Coeff::Rat(n1, d1), Coeff::Rat(n2, d2)) => {
                Coeff::Rat(n1 * d2 + n2 * d1, d1 * d2).reduce()
            }
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Self) -> Self {
        match (self, other) {
            (Coeff::Int(a), Coeff::Int(b)) => Coeff::Int(a * b),
            (Coeff::Int(a), Coeff::Rat(n, d)) | (Coeff::Rat(n, d), Coeff::Int(a)) => {
                Coeff::Rat(a * n, *d).reduce()
            }
            (Coeff::Rat(n1, d1), Coeff::Rat(n2, d2)) => {
                Coeff::Rat(n1 * n2, d1 * d2).reduce()
            }
        }
    }

    pub fn div(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (_, c) if c.is_zero() => None,
            (Coeff::Int(a), Coeff::Int(b)) => {
                if a % b == 0 {
                    Some(Coeff::Int(a / b))
                } else {
                    Some(Coeff::Rat(*a, *b).reduce())
                }
            }
            (Coeff::Int(a), Coeff::Rat(n, d)) => {
                Some(Coeff::Rat(a * d, *n).reduce())
            }
            (Coeff::Rat(n, d), Coeff::Int(b)) => {
                Some(Coeff::Rat(*n, d * b).reduce())
            }
            (Coeff::Rat(n1, d1), Coeff::Rat(n2, d2)) => {
                Some(Coeff::Rat(n1 * d2, d1 * n2).reduce())
            }
        }
    }

    pub fn abs(&self) -> Self {
        match self {
            Coeff::Int(n) => Coeff::Int(n.abs()),
            Coeff::Rat(n, d) => Coeff::Rat(n.abs(), d.abs()),
        }
    }

    fn reduce(self) -> Self {
        match self {
            Coeff::Rat(n, d) => {
                if d == 0 { return Coeff::Int(0); }
                let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i64;
                let (n, d) = (n / g, d / g);
                let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
                if d == 1 { Coeff::Int(n) } else { Coeff::Rat(n, d) }
            }
            other => other,
        }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

impl fmt::Display for Coeff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Coeff::Int(n) => write!(f, "{}", n),
            Coeff::Rat(n, d) => write!(f, "{}/{}", n, d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coeff_add() {
        assert_eq!(Coeff::Int(3).add(&Coeff::Int(4)), Coeff::Int(7));
        assert_eq!(Coeff::Rat(1, 3).add(&Coeff::Rat(1, 6)), Coeff::Rat(1, 2));
        assert_eq!(Coeff::Int(1).add(&Coeff::Rat(1, 2)), Coeff::Rat(3, 2));
    }

    #[test]
    fn coeff_mul() {
        assert_eq!(Coeff::Int(3).mul(&Coeff::Int(4)), Coeff::Int(12));
        assert_eq!(Coeff::Rat(2, 3).mul(&Coeff::Rat(3, 4)), Coeff::Rat(1, 2));
    }

    #[test]
    fn coeff_div() {
        assert_eq!(Coeff::Int(6).div(&Coeff::Int(3)), Some(Coeff::Int(2)));
        assert_eq!(Coeff::Int(1).div(&Coeff::Int(3)), Some(Coeff::Rat(1, 3)));
        assert_eq!(Coeff::Int(5).div(&Coeff::Int(0)), None);
    }

    #[test]
    fn coeff_neg() {
        assert_eq!(Coeff::Int(3).neg(), Coeff::Int(-3));
        assert_eq!(Coeff::Rat(1, 2).neg(), Coeff::Rat(-1, 2));
    }

    #[test]
    fn coeff_reduce() {
        assert_eq!(Coeff::Rat(6, 4).reduce(), Coeff::Rat(3, 2));
        assert_eq!(Coeff::Rat(4, 2).reduce(), Coeff::Int(2));
        assert_eq!(Coeff::Rat(-3, -6).reduce(), Coeff::Rat(1, 2));
    }

    // --- Comprehensive coeff tests ---

    #[test]
    fn coeff_zero_one() {
        assert!(Coeff::zero().is_zero());
        assert!(Coeff::one().is_one());
        assert!(!Coeff::Int(2).is_zero());
        assert!(!Coeff::Int(2).is_one());
        assert!(Coeff::Rat(0, 5).is_zero());
        assert!(Coeff::Rat(3, 3).is_one());
    }

    #[test]
    fn coeff_add_identity() {
        let c = Coeff::Int(42);
        assert_eq!(c.add(&Coeff::zero()), c);
        assert_eq!(Coeff::zero().add(&c), c);
    }

    #[test]
    fn coeff_mul_identity() {
        let c = Coeff::Rat(3, 7);
        assert_eq!(c.mul(&Coeff::one()), c);
        assert_eq!(Coeff::one().mul(&c), c);
    }

    #[test]
    fn coeff_mul_zero() {
        assert_eq!(Coeff::Int(42).mul(&Coeff::zero()), Coeff::zero());
    }

    #[test]
    fn coeff_sub() {
        assert_eq!(Coeff::Int(5).sub(&Coeff::Int(3)), Coeff::Int(2));
        assert_eq!(Coeff::Rat(1, 2).sub(&Coeff::Rat(1, 3)), Coeff::Rat(1, 6));
    }

    #[test]
    fn coeff_div_rat() {
        assert_eq!(
            Coeff::Rat(2, 3).div(&Coeff::Rat(4, 5)),
            Some(Coeff::Rat(5, 6))
        );
    }

    #[test]
    fn coeff_abs() {
        assert_eq!(Coeff::Int(-5).abs(), Coeff::Int(5));
        assert_eq!(Coeff::Int(5).abs(), Coeff::Int(5));
        assert_eq!(Coeff::Rat(-3, 4).abs(), Coeff::Rat(3, 4));
    }

    #[test]
    fn coeff_neg_double() {
        let c = Coeff::Rat(3, 7);
        assert_eq!(c.neg().neg(), c);
    }

    #[test]
    fn coeff_display() {
        assert_eq!(Coeff::Int(42).to_string(), "42");
        assert_eq!(Coeff::Rat(3, 4).to_string(), "3/4");
    }

    #[test]
    fn coeff_reduce_negative_denom() {
        assert_eq!(Coeff::Rat(3, -4).reduce(), Coeff::Rat(-3, 4));
    }
}
