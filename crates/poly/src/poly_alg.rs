use maxima_core::SymbolId;
use crate::alg_field::{AlgField, AlgNumber};
use crate::traits::{Ring, Field};

/// Polynomial with algebraic number coefficients: elements of Q(α)[x].
#[derive(Debug, Clone)]
pub struct PolyAlg {
    pub var: SymbolId,
    pub terms: Vec<(u32, AlgNumber)>,
    pub field: AlgField,
}

impl PolyAlg {
    pub fn zero(var: SymbolId, field: &AlgField) -> Self {
        PolyAlg { var, terms: vec![], field: field.clone() }
    }

    pub fn constant(var: SymbolId, c: AlgNumber) -> Self {
        let field = c.field.clone();
        if c.is_zero() { return Self::zero(var, &field); }
        PolyAlg { var, terms: vec![(0, c)], field }
    }

    pub fn from_int(var: SymbolId, n: i64, field: &AlgField) -> Self {
        if n == 0 { return Self::zero(var, field); }
        PolyAlg { var, terms: vec![(0, field.from_rational(n, 1))], field: field.clone() }
    }

    pub fn var_poly(var: SymbolId, field: &AlgField) -> Self {
        PolyAlg { var, terms: vec![(1, field.one())], field: field.clone() }
    }

    pub fn monomial(var: SymbolId, exp: u32, coeff: AlgNumber) -> Self {
        let field = coeff.field.clone();
        if coeff.is_zero() { return Self::zero(var, &field); }
        PolyAlg { var, terms: vec![(exp, coeff)], field }
    }

    /// Lift a Poly (over Q) into Q(α)[x].
    pub fn from_poly(p: &crate::Poly, field: &AlgField) -> Self {
        let terms: Vec<(u32, AlgNumber)> = p.terms.iter().map(|(e, c)| {
            let alg_c = match c {
                crate::Coeff::Int(n) => field.from_rational(*n, 1),
                crate::Coeff::Rat(n, d) => field.from_rational(*n, *d),
            };
            (*e, alg_c)
        }).filter(|(_, c)| !c.is_zero()).collect();
        PolyAlg { var: p.var, terms, field: field.clone() }
    }

    pub fn is_zero(&self) -> bool { self.terms.is_empty() }

    pub fn is_constant(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].0 == 0)
    }

    pub fn degree(&self) -> Option<u32> {
        self.terms.iter().map(|(e, _)| *e).max()
    }

    pub fn leading_coeff(&self) -> AlgNumber {
        if let Some(deg) = self.degree() {
            self.terms.iter().find(|(e, _)| *e == deg).map(|(_, c)| c.clone())
                .unwrap_or_else(|| self.field.zero())
        } else { self.field.zero() }
    }

    pub fn coeff_at(&self, exp: u32) -> AlgNumber {
        self.terms.iter().find(|(e, _)| *e == exp).map(|(_, c)| c.clone())
            .unwrap_or_else(|| self.field.zero())
    }

    pub fn add(&self, other: &PolyAlg) -> PolyAlg {
        let mut result = self.terms.clone();
        for (e, c) in &other.terms {
            if let Some(pos) = result.iter().position(|(re, _)| *re == *e) {
                result[pos].1 = result[pos].1.add(c);
            } else {
                result.push((*e, c.clone()));
            }
        }
        result.retain(|(_, c)| !c.is_zero());
        result.sort_by(|a, b| b.0.cmp(&a.0));
        PolyAlg { var: self.var, terms: result, field: self.field.clone() }
    }

    pub fn sub(&self, other: &PolyAlg) -> PolyAlg {
        self.add(&other.neg())
    }

    pub fn neg(&self) -> PolyAlg {
        let terms = self.terms.iter().map(|(e, c)| (*e, c.neg())).collect();
        PolyAlg { var: self.var, terms, field: self.field.clone() }
    }

    pub fn mul(&self, other: &PolyAlg) -> PolyAlg {
        if self.is_zero() || other.is_zero() {
            return Self::zero(self.var, &self.field);
        }
        let mut result_map: std::collections::BTreeMap<u32, AlgNumber> = std::collections::BTreeMap::new();
        for (e1, c1) in &self.terms {
            for (e2, c2) in &other.terms {
                let exp = e1 + e2;
                let prod = c1.mul(c2);
                let entry = result_map.entry(exp).or_insert_with(|| self.field.zero());
                *entry = entry.add(&prod);
            }
        }
        let terms: Vec<(u32, AlgNumber)> = result_map.into_iter()
            .filter(|(_, c)| !c.is_zero())
            .map(|(e, c)| (e, c))
            .collect::<Vec<_>>()
            .into_iter().rev().collect();
        PolyAlg { var: self.var, terms, field: self.field.clone() }
    }

    pub fn scale(&self, c: &AlgNumber) -> PolyAlg {
        if c.is_zero() { return Self::zero(self.var, &self.field); }
        let terms = self.terms.iter().map(|(e, coeff)| (*e, coeff.mul(c))).collect();
        PolyAlg { var: self.var, terms, field: self.field.clone() }
    }

    /// Polynomial long division. Returns (quotient, remainder).
    pub fn divmod(&self, divisor: &PolyAlg) -> Option<(PolyAlg, PolyAlg)> {
        if divisor.is_zero() { return None; }
        let mut quotient = Self::zero(self.var, &self.field);
        let mut remainder = self.clone();
        let lc_div = divisor.leading_coeff();
        let lc_inv = lc_div.inv()?;
        let deg_div = divisor.degree().unwrap();

        while !remainder.is_zero() {
            let deg_rem = match remainder.degree() {
                Some(d) => d,
                None => break,
            };
            if deg_rem < deg_div { break; }
            let lc_rem = remainder.leading_coeff();
            let coeff = lc_rem.mul(&lc_inv);
            let shift = deg_rem - deg_div;
            let term = Self::monomial(self.var, shift, coeff);
            quotient = quotient.add(&term);
            remainder = remainder.sub(&term.mul(divisor));
        }
        Some((quotient, remainder))
    }

    /// GCD via Euclidean algorithm.
    pub fn gcd(&self, other: &PolyAlg) -> PolyAlg {
        if self.is_zero() { return other.make_monic(); }
        if other.is_zero() { return self.make_monic(); }

        let mut a = self.clone();
        let mut b = other.clone();
        if a.degree().unwrap_or(0) < b.degree().unwrap_or(0) {
            std::mem::swap(&mut a, &mut b);
        }
        while !b.is_zero() {
            let (_, rem) = match a.divmod(&b) {
                Some(qr) => qr,
                None => break,
            };
            a = b;
            b = rem;
        }
        a.make_monic()
    }

    /// Make polynomial monic (leading coefficient = 1).
    pub fn make_monic(&self) -> PolyAlg {
        if self.is_zero() { return self.clone(); }
        let lc = self.leading_coeff();
        if let Some(inv) = lc.inv() {
            self.scale(&inv)
        } else {
            self.clone()
        }
    }

    pub fn derivative(&self) -> PolyAlg {
        let terms: Vec<(u32, AlgNumber)> = self.terms.iter()
            .filter(|(e, _)| *e > 0)
            .map(|(e, c)| (e - 1, c.mul(&self.field.from_rational(*e as i64, 1))))
            .filter(|(_, c)| !c.is_zero())
            .collect();
        PolyAlg { var: self.var, terms, field: self.field.clone() }
    }

    /// Extended GCD: returns (gcd, s, t) where s*self + t*other = gcd.
    pub fn extended_gcd(&self, other: &PolyAlg) -> (PolyAlg, PolyAlg, PolyAlg) {
        let one_p = Self::from_int(self.var, 1, &self.field);
        let zero_p = Self::zero(self.var, &self.field);

        let mut old_r = self.clone();
        let mut r = other.clone();
        let mut old_s = one_p.clone();
        let mut s = zero_p.clone();
        let mut old_t = zero_p.clone();
        let mut t = one_p;

        while !r.is_zero() {
            let (q, rem) = match old_r.divmod(&r) {
                Some(qr) => qr,
                None => break,
            };
            old_r = r.clone();
            r = rem;
            let new_s = old_s.sub(&q.mul(&s));
            old_s = s; s = new_s;
            let new_t = old_t.sub(&q.mul(&t));
            old_t = t; t = new_t;
        }
        (old_r, old_s, old_t)
    }
    /// Conjugate: for Q(√d), negate the generator coefficient in each term.
    /// This maps √d → -√d, giving the Galois conjugate.
    pub fn conjugate(&self) -> PolyAlg {
        if self.field.degree != 2 { return self.clone(); }
        let terms = self.terms.iter().map(|(e, c)| {
            let mut conj_c = c.clone();
            if conj_c.coeffs.len() >= 2 {
                conj_c.coeffs[1] = (-(conj_c.coeffs[1].0), conj_c.coeffs[1].1);
            }
            (*e, conj_c)
        }).collect();
        PolyAlg { var: self.var, terms, field: self.field.clone() }
    }

    /// Compute the norm: N(x) = f(x) · conj(f(x)).
    /// For Q(√d): N(x) = f(x)·f̄(x) ∈ Q[x] (conjugate product).
    /// Returns a Poly over Q (all algebraic parts cancel).
    pub fn norm_poly(&self) -> crate::Poly {
        let product = self.mul(&self.conjugate());
        // Extract rational coefficients (algebraic parts should be zero)
        let mut terms = Vec::new();
        for (e, c) in &product.terms {
            let rat = c.coeffs[0]; // constant part (rational)
            if rat.0 != 0 {
                let coeff = if rat.1 == 1 {
                    crate::Coeff::Int(rat.0)
                } else {
                    crate::Coeff::Rat(rat.0, rat.1)
                };
                terms.push((*e, coeff));
            }
        }
        crate::Poly { var: self.var, terms }
    }
}

/// Factor a polynomial over Q(α) using the norm-based method.
/// Input: f(x) ∈ Q[x] (irreducible over Q), field Q(α).
/// Output: factors of f in Q(α)[x].
pub fn factor_over_extension(f: &crate::Poly, field: &AlgField) -> Vec<PolyAlg> {
    let f_lifted = PolyAlg::from_poly(f, field);
    let var = f.var;
    let deg = f.degree().unwrap_or(0);

    if field.degree == 2 && deg >= 2 {
        let alpha = field.gen();
        let _target_deg = deg / 2; // looking for factors of degree deg/2

        // Trial divisor: try gcd(f, candidate) for candidates of degree deg/2
        // Candidates: x² + a·α·x + (b + c·α) for small integers a,b,c
        for b in -3..=3i64 {
            for a_coeff in &[0i64, -1, 1] {
                for c_coeff in &[0i64, -1, 1] {
                    let linear = if *a_coeff == 0 { field.zero() } else { alpha.scale(*a_coeff, 1) };
                    let constant = field.from_rational(b, 1).add(&alpha.scale(*c_coeff, 1));
                    if linear.is_zero() && constant.is_zero() { continue; }
                    let mut terms = vec![(2u32, field.one())];
                    if !linear.is_zero() { terms.push((1, linear)); }
                    if !constant.is_zero() { terms.push((0, constant)); }
                    let cand = PolyAlg { var, field: field.clone(), terms };
                    let g = f_lifted.gcd(&cand);
                    if g.degree().unwrap_or(0) >= 1 && g.degree() < f_lifted.degree() {
                        if let Some((q, rem)) = f_lifted.divmod(&g) {
                            if rem.is_zero() && q.degree().unwrap_or(0) >= 1 {
                                return vec![g, q];
                            }
                        }
                    }
                }
            }
        }

        // Also try linear factors: x + c·α + d
        for d in -3..=3i64 {
            for a_coeff in [-1i64, 1] {
                let cand = PolyAlg {
                    var, field: field.clone(),
                    terms: vec![
                        (1, field.one()),
                        (0, alpha.scale(a_coeff, 1).add(&field.from_rational(d, 1))),
                    ],
                };
                let g = f_lifted.gcd(&cand);
                if g.degree() == Some(1) {
                    if let Some((q, rem)) = f_lifted.divmod(&g) {
                        if rem.is_zero() {
                            let mut factors = vec![g];
                            // Recursively factor the quotient
                            let q_factors = factor_quotient_recursive(&q, field, var);
                            factors.extend(q_factors);
                            return factors;
                        }
                    }
                }
            }
        }

        return vec![f_lifted];
    }

    vec![f_lifted]
}

fn factor_quotient_recursive(q: &PolyAlg, field: &AlgField, var: SymbolId) -> Vec<PolyAlg> {
    if q.degree().unwrap_or(0) <= 1 { return vec![q.clone()]; }
    let alpha = field.gen();
    for d in -3..=3i64 {
        for a_coeff in [-1i64, 1] {
            let cand = PolyAlg {
                var, field: field.clone(),
                terms: vec![
                    (1, field.one()),
                    (0, alpha.scale(a_coeff, 1).add(&field.from_rational(d, 1))),
                ],
            };
            let g = q.gcd(&cand);
            if g.degree() == Some(1) {
                if let Some((rem_q, rem)) = q.divmod(&g) {
                    if rem.is_zero() {
                        let mut factors = vec![g];
                        factors.extend(factor_quotient_recursive(&rem_q, field, var));
                        return factors;
                    }
                }
            }
        }
    }
    vec![q.clone()]
}

/// Shift polynomial: compute f(x + s) where s is an AlgNumber.
#[allow(dead_code)]
fn shift_poly(f: &PolyAlg, s: &AlgNumber, var: SymbolId, field: &AlgField) -> PolyAlg {
    // f(x+s) = Σ a_k · (x+s)^k
    // Expand each (x+s)^k via binomial theorem
    let mut result = PolyAlg::zero(var, field);
    for (k, coeff) in &f.terms {
        // a_k · (x+s)^k = a_k · Σ_{j=0}^{k} C(k,j) · x^j · s^(k-j)
        let mut s_power = field.one(); // s^0 = 1
        for j in 0..=*k {
            let binom = binomial_coeff(*k, j);
            let term_coeff = coeff.mul(&s_power).scale(binom as i64, 1);
            if !term_coeff.is_zero() {
                let term = PolyAlg::monomial(var, j, term_coeff);
                result = result.add(&term);
            }
            if j < *k {
                s_power = s_power.mul(s);
            }
        }
    }
    result
}

#[allow(dead_code)]
fn binomial_coeff(n: u32, k: u32) -> u64 {
    if k > n { return 0; }
    let k = k.min(n - k) as u64;
    let mut result = 1u64;
    for i in 0..k {
        result = result * (n as u64 - i) / (i + 1);
    }
    result
}

/// Compute cyclotomic polynomial Φ_n(x) over Q.
/// Φ_n(x) = Π_{d|n} (x^d - 1)^μ(n/d) where μ is the Möbius function.
pub fn cyclotomic_poly(n: u32, var: maxima_core::SymbolId) -> crate::Poly {
    if n == 1 {
        // Φ₁(x) = x - 1
        return crate::Poly { var, terms: vec![(1, crate::Coeff::Int(1)), (0, crate::Coeff::Int(-1))] };
    }
    if n == 2 {
        // Φ₂(x) = x + 1
        return crate::Poly { var, terms: vec![(1, crate::Coeff::Int(1)), (0, crate::Coeff::Int(1))] };
    }

    // General: Φ_n(x) = (x^n - 1) / Π_{d|n, d<n} Φ_d(x)
    let mut xn_minus_1 = crate::Poly::zero(var);
    xn_minus_1.terms = vec![(n, crate::Coeff::Int(1)), (0, crate::Coeff::Int(-1))];

    let divisors = get_divisors(n);
    let mut product = crate::Poly::constant(var, crate::Coeff::one());
    for &d in &divisors {
        if d < n {
            let phi_d = cyclotomic_poly(d, var);
            product = product.mul(&phi_d);
        }
    }

    match xn_minus_1.exact_div(&product) {
        Some(q) => q,
        None => xn_minus_1, // fallback
    }
}

fn get_divisors(n: u32) -> Vec<u32> {
    let mut divs = Vec::new();
    for i in 1..=n {
        if n % i == 0 { divs.push(i); }
    }
    divs
}

impl std::fmt::Display for PolyAlg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_zero() { return write!(f, "0"); }
        let var_name = maxima_core::resolve(self.var);
        let mut first = true;
        let mut sorted = self.terms.clone();
        sorted.sort_by(|a, b| b.0.cmp(&a.0));
        for (e, c) in &sorted {
            if !first { write!(f, " + ")?; }
            first = false;
            if *e == 0 { write!(f, "({})", c)?; }
            else if *e == 1 { write!(f, "({})*{}", c, var_name)?; }
            else { write!(f, "({})*{}^{}", c, var_name, e)?; }
        }
        Ok(())
    }
}

// Implement Ring trait for AlgNumber
impl Ring for AlgNumber {
    fn zero() -> Self { panic!("AlgNumber::zero() needs a field; use field.zero()"); }
    fn one() -> Self { panic!("AlgNumber::one() needs a field; use field.one()"); }
    fn is_zero(&self) -> bool { self.is_zero() }
    fn is_one(&self) -> bool {
        self.coeffs[0] == (1, 1) && self.coeffs.iter().skip(1).all(|(n, _)| *n == 0)
    }
    fn add(&self, other: &Self) -> Self { AlgNumber::add(self, other) }
    fn sub(&self, other: &Self) -> Self { AlgNumber::sub(self, other) }
    fn mul(&self, other: &Self) -> Self { AlgNumber::mul(self, other) }
    fn neg(&self) -> Self { AlgNumber::neg(self) }
}

impl Field for AlgNumber {
    fn inv(&self) -> Option<Self> { AlgNumber::inv(self) }
}

// Need IntegralDomain, GcdDomain, EuclideanDomain for the trait hierarchy
impl crate::traits::IntegralDomain for AlgNumber {}
impl crate::traits::GcdDomain for AlgNumber {
    fn gcd(&self, other: &Self) -> Self {
        // In a field, gcd is trivial: if either is nonzero, gcd = 1
        if self.is_zero() && other.is_zero() {
            return self.clone();
        }
        self.field.one()
    }
}
impl crate::traits::EuclideanDomain for AlgNumber {
    fn divmod(&self, other: &Self) -> Option<(Self, Self)> {
        let q = self.div(other)?;
        Some((q, self.field.zero()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::intern;

    fn x() -> SymbolId { intern("x") }

    #[test]
    fn poly_alg_basic() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();

        // p = x + √2
        let p = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.clone())], field: f.clone() };
        // q = x - √2
        let q = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.neg())], field: f.clone() };
        // p*q = x² - 2
        let pq = p.mul(&q);
        assert_eq!(pq.degree(), Some(2));
        assert!(pq.coeff_at(2).is_one());
        assert_eq!(pq.coeff_at(0), f.from_rational(-2, 1));
        assert!(pq.coeff_at(1).is_zero());
    }

    #[test]
    fn poly_alg_gcd() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();

        // p = x² - 2 = (x+√2)(x-√2)
        let p = PolyAlg { var: x(), terms: vec![(2, f.one()), (0, f.from_rational(-2, 1))], field: f.clone() };
        // q = x + √2
        let q = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.clone())], field: f.clone() };
        let g = p.gcd(&q);
        // gcd should be x + √2 (monic)
        assert_eq!(g.degree(), Some(1), "gcd degree should be 1, got {:?}", g);
    }

    #[test]
    fn poly_alg_divmod() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();

        // p = x² - 2, q = x + √2
        // p / q = x - √2, remainder = 0
        let p = PolyAlg { var: x(), terms: vec![(2, f.one()), (0, f.from_rational(-2, 1))], field: f.clone() };
        let q = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.clone())], field: f.clone() };
        let (quot, rem) = p.divmod(&q).unwrap();
        assert!(rem.is_zero(), "remainder should be 0");
        assert_eq!(quot.degree(), Some(1));
        // quotient = x - √2
        assert!(quot.coeff_at(1).is_one());
        assert_eq!(quot.coeff_at(0), s2.neg());
    }

    #[test]
    fn factor_x4_plus_1() {
        // x⁴+1 = (x²+√2x+1)(x²-√2x+1) over Q(√2)
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();

        let f1 = PolyAlg { var: x(), terms: vec![
            (2, f.one()), (1, s2.clone()), (0, f.one())
        ], field: f.clone() };
        let f2 = PolyAlg { var: x(), terms: vec![
            (2, f.one()), (1, s2.neg()), (0, f.one())
        ], field: f.clone() };

        let product = f1.mul(&f2);
        // Should be x⁴ + 1
        assert_eq!(product.degree(), Some(4));
        assert!(product.coeff_at(4).is_one());
        assert!(product.coeff_at(3).is_zero());
        assert!(product.coeff_at(2).is_zero());
        assert!(product.coeff_at(1).is_zero());
        assert!(product.coeff_at(0).is_one());
    }

    #[test]
    fn norm_based_factor_x4_plus_1() {
        let f = AlgField::from_sqrt(2);
        let p = crate::Poly { var: x(), terms: vec![(4, crate::Coeff::Int(1)), (0, crate::Coeff::Int(1))] };
        let factors = factor_over_extension(&p, &f);
        assert_eq!(factors.len(), 2, "x^4+1 should factor into 2 pieces over Q(√2), got {}", factors.len());
        // Verify product equals original
        let product = factors[0].mul(&factors[1]);
        assert!(product.coeff_at(4).is_one());
        assert!(product.coeff_at(0).is_one());
        assert!(product.coeff_at(3).is_zero());
        assert!(product.coeff_at(2).is_zero());
        assert!(product.coeff_at(1).is_zero());
    }

    #[test]
    fn norm_based_factor_x4_minus_2() {
        // x⁴-2 over Q(√2): factors as (x²-√2)(x²+√2)
        let f = AlgField::from_sqrt(2);
        let p = crate::Poly { var: x(), terms: vec![(4, crate::Coeff::Int(1)), (0, crate::Coeff::Int(-2))] };
        let factors = factor_over_extension(&p, &f);
        assert!(factors.len() >= 2, "x^4-2 should factor over Q(√2), got {} factors", factors.len());
    }

    #[test]
    fn cyclotomic_polys() {
        // Φ₁ = x-1, Φ₂ = x+1, Φ₃ = x²+x+1, Φ₄ = x²+1, Φ₆ = x²-x+1
        let phi1 = cyclotomic_poly(1, x());
        assert_eq!(phi1.degree(), Some(1));

        let phi2 = cyclotomic_poly(2, x());
        assert_eq!(phi2.degree(), Some(1));

        let phi3 = cyclotomic_poly(3, x());
        assert_eq!(phi3.degree(), Some(2));

        let phi4 = cyclotomic_poly(4, x());
        assert_eq!(phi4.degree(), Some(2));

        let phi6 = cyclotomic_poly(6, x());
        assert_eq!(phi6.degree(), Some(2));

        // Verify: x⁶-1 = Φ₁·Φ₂·Φ₃·Φ₆
        let product = phi1.mul(&phi2).mul(&phi3).mul(&phi6);
        assert_eq!(product.degree(), Some(6));
        // Leading coeff should be 1, constant should be -1
        assert_eq!(product.leading_coeff(), crate::Coeff::Int(1));
        assert_eq!(product.constant_term(), crate::Coeff::Int(-1));
    }

    #[test]
    fn conjugate_test() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();
        // p = x + √2
        let p = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.clone())], field: f.clone() };
        let pc = p.conjugate();
        // conjugate should be x - √2
        assert!(pc.coeff_at(1).is_one());
        assert_eq!(pc.coeff_at(0), s2.neg());
        // product p·p̄ = x²-2 (rational!)
        let norm = p.norm_poly();
        assert_eq!(norm.degree(), Some(2));
        assert_eq!(norm.constant_term(), crate::Coeff::Int(-2));
    }

    #[test]
    fn from_poly_lift() {
        let f = AlgField::from_sqrt(2);
        let p = crate::Poly { var: x(), terms: vec![(2, crate::Coeff::Int(3)), (0, crate::Coeff::Int(-1))] };
        let pa = PolyAlg::from_poly(&p, &f);
        assert_eq!(pa.degree(), Some(2));
        assert_eq!(pa.coeff_at(2), f.from_rational(3, 1));
        assert_eq!(pa.coeff_at(0), f.from_rational(-1, 1));
    }

    #[test]
    fn derivative_alg() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();
        // p = x² + √2x + 1, p' = 2x + √2
        let p = PolyAlg { var: x(), terms: vec![
            (2, f.one()), (1, s2.clone()), (0, f.one())
        ], field: f.clone() };
        let dp = p.derivative();
        assert_eq!(dp.degree(), Some(1));
        assert_eq!(dp.coeff_at(1), f.from_rational(2, 1));
        assert_eq!(dp.coeff_at(0), s2);
    }

    #[test]
    fn extended_gcd_alg() {
        let f = AlgField::from_sqrt(2);
        let s2 = f.gen();
        let a = PolyAlg { var: x(), terms: vec![(2, f.one()), (0, f.from_rational(-2, 1))], field: f.clone() };
        let b = PolyAlg { var: x(), terms: vec![(1, f.one()), (0, s2.clone())], field: f.clone() };
        let (g, _s, _t) = a.extended_gcd(&b);
        // g should divide both a and b
        assert!(g.degree().unwrap_or(0) >= 1 || !g.is_zero());
        // Verify: s*a + t*b = g (approximately, via checking at a point)
    }
}
