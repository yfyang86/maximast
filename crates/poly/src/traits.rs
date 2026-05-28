/// Algebraic structure traits — the FriCAS-style category hierarchy in Rust.
///
/// These traits define the mathematical structures that algorithms
/// are generic over. A polynomial GCD algorithm works over any GcdDomain;
/// the Risch algorithm works over any DifferentialField.

use maxima_core::SymbolId;

/// A ring with additive and multiplicative identity.
pub trait Ring: Clone + PartialEq + std::fmt::Debug {
    fn zero() -> Self;
    fn one() -> Self;
    fn is_zero(&self) -> bool;
    fn is_one(&self) -> bool;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn neg(&self) -> Self;
}

/// An integral domain: a commutative ring with no zero divisors.
pub trait IntegralDomain: Ring {}

/// A GCD domain: an integral domain where GCD is defined.
pub trait GcdDomain: IntegralDomain {
    fn gcd(&self, other: &Self) -> Self;
    fn content_wrt(&self, _var: SymbolId) -> Self { self.clone() }
}

/// A Euclidean domain: a GCD domain with division with remainder.
pub trait EuclideanDomain: GcdDomain {
    fn divmod(&self, other: &Self) -> Option<(Self, Self)>;
    fn exact_div(&self, other: &Self) -> Option<Self> {
        let (q, r) = self.divmod(other)?;
        if r.is_zero() { Some(q) } else { None }
    }
}

/// A field: a Euclidean domain where every non-zero element has an inverse.
pub trait Field: EuclideanDomain {
    fn inv(&self) -> Option<Self>;
    fn div(&self, other: &Self) -> Option<Self> {
        let inv_other = other.inv()?;
        Some(self.mul(&inv_other))
    }
}

/// A differential ring: a ring with a derivation operation.
pub trait DifferentialRing: Ring {
    fn deriv(&self, var: SymbolId) -> Self;
}

/// A differential field: a field with derivation.
pub trait DifferentialField: Field + DifferentialRing {}

// === Implementations for basic types ===

impl Ring for i64 {
    fn zero() -> Self { 0 }
    fn one() -> Self { 1 }
    fn is_zero(&self) -> bool { *self == 0 }
    fn is_one(&self) -> bool { *self == 1 }
    fn add(&self, other: &Self) -> Self { self + other }
    fn sub(&self, other: &Self) -> Self { self - other }
    fn mul(&self, other: &Self) -> Self { self * other }
    fn neg(&self) -> Self { -self }
}

impl IntegralDomain for i64 {}

impl GcdDomain for i64 {
    fn gcd(&self, other: &Self) -> Self {
        let (mut a, mut b) = (self.abs(), other.abs());
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }
}

impl EuclideanDomain for i64 {
    fn divmod(&self, other: &Self) -> Option<(Self, Self)> {
        if *other == 0 { return None; }
        Some((self / other, self % other))
    }
}

// Implementation for our Coeff type
impl Ring for crate::Coeff {
    fn zero() -> Self { crate::Coeff::Int(0) }
    fn one() -> Self { crate::Coeff::Int(1) }
    fn is_zero(&self) -> bool { crate::Coeff::is_zero(self) }
    fn is_one(&self) -> bool { crate::Coeff::is_one(self) }
    fn add(&self, other: &Self) -> Self { crate::Coeff::add(self, other) }
    fn sub(&self, other: &Self) -> Self { crate::Coeff::sub(self, other) }
    fn mul(&self, other: &Self) -> Self { crate::Coeff::mul(self, other) }
    fn neg(&self) -> Self { crate::Coeff::neg(self) }
}

impl IntegralDomain for crate::Coeff {}

impl GcdDomain for crate::Coeff {
    fn gcd(&self, other: &Self) -> Self {
        match (self, other) {
            (crate::Coeff::Int(a), crate::Coeff::Int(b)) => {
                let g = <i64 as GcdDomain>::gcd(a, b);
                crate::Coeff::Int(g)
            }
            _ => crate::Coeff::one(),
        }
    }
}

impl EuclideanDomain for crate::Coeff {
    fn divmod(&self, other: &Self) -> Option<(Self, Self)> {
        crate::Coeff::div(self, other).map(|q| (q, Self::zero()))
    }
}

impl Field for crate::Coeff {
    fn inv(&self) -> Option<Self> {
        if self.is_zero() { return None; }
        crate::Coeff::div(&crate::Coeff::one(), self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_ring() {
        assert_eq!(<i64 as Ring>::zero(), 0);
        assert_eq!(<i64 as Ring>::one(), 1);
        assert_eq!(Ring::add(&3i64, &4), 7);
        assert_eq!(Ring::mul(&3i64, &4), 12);
    }

    #[test]
    fn i64_gcd() {
        assert_eq!(GcdDomain::gcd(&12i64, &8), 4);
        assert_eq!(GcdDomain::gcd(&7i64, &11), 1);
    }

    #[test]
    fn i64_euclidean() {
        assert_eq!(EuclideanDomain::divmod(&10i64, &3), Some((3, 1)));
        assert_eq!(EuclideanDomain::divmod(&10i64, &0), None);
    }

    #[test]
    fn coeff_ring() {
        use crate::Coeff;
        assert!(Ring::is_zero(&Coeff::Int(0)));
        assert!(Ring::is_one(&Coeff::Int(1)));
        assert_eq!(Ring::add(&Coeff::Int(3), &Coeff::Int(4)), Coeff::Int(7));
    }

    #[test]
    fn coeff_field() {
        use crate::Coeff;
        let inv = Field::inv(&Coeff::Int(2));
        assert_eq!(inv, Some(Coeff::Rat(1, 2)));
    }
}
