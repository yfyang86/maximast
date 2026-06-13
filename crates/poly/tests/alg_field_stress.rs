use maxima_poly::alg_field::AlgField;

// ============================================================
// V3.1 STRESS TESTS — verify algebraic number field is real,
// not hardcoded for sqrt(2) or specific cases
// ============================================================

// --- Different square root fields ---

#[test]
fn sqrt3_squared() {
    let f = AlgField::from_sqrt(3);
    let s = f.gen();
    let p = s.mul(&s);
    assert_eq!(p.coeffs[0], (3, 1), "√3·√3 should be 3");
    assert_eq!(p.coeffs[1], (0, 1));
}

#[test]
fn sqrt5_squared() {
    let f = AlgField::from_sqrt(5);
    let s = f.gen();
    assert_eq!(s.mul(&s).coeffs[0], (5, 1));
}

#[test]
fn sqrt7_inverse() {
    let f = AlgField::from_sqrt(7);
    let s = f.gen();
    let inv = s.inv().unwrap();
    // 1/√7 = √7/7
    assert_eq!(inv.coeffs[0], (0, 1));
    assert_eq!(inv.coeffs[1], (1, 7));
    // verify: √7 * (√7/7) = 7/7 = 1
    let prod = s.mul(&inv);
    assert_eq!(prod.coeffs[0], (1, 1));
    assert_eq!(prod.coeffs[1], (0, 1));
}

#[test]
fn sqrt11_conjugate_product() {
    // (a + b√11)(a - b√11) = a² - 11b²
    let f = AlgField::from_sqrt(11);
    let s = f.gen();
    let a = f.from_rational(3, 1);
    let b_s = s.scale(2, 1); // 2√11
    let plus = a.add(&b_s);   // 3 + 2√11
    let minus = a.sub(&b_s);  // 3 - 2√11
    let prod = plus.mul(&minus);
    // 9 - 4*11 = 9 - 44 = -35
    assert_eq!(prod.coeffs[0], (-35, 1));
    assert_eq!(prod.coeffs[1], (0, 1));
}

#[test]
fn sqrt_neg2_field() {
    // Q(√(-2)) = Q(i√2)
    let f = AlgField::from_sqrt(-2);
    let s = f.gen(); // i√2
    let p = s.mul(&s);
    assert_eq!(p.coeffs[0], (-2, 1), "(i√2)² = -2");
}

// --- Inverse for various elements ---

#[test]
fn inv_2_plus_3sqrt5() {
    let f = AlgField::from_sqrt(5);
    let s = f.gen();
    let a = f.from_rational(2, 1).add(&s.scale(3, 1)); // 2 + 3√5
    let inv = a.inv().unwrap();
    let prod = a.mul(&inv);
    assert_eq!(prod.coeffs[0], (1, 1), "a * a^(-1) should be 1, got {:?}", prod.coeffs);
    assert_eq!(prod.coeffs[1], (0, 1));
}

#[test]
fn inv_rational_in_field() {
    // 1/(3/4) = 4/3 even inside algebraic field
    let f = AlgField::from_sqrt(2);
    let a = f.from_rational(3, 4);
    let inv = a.inv().unwrap();
    assert_eq!(inv.coeffs[0], (4, 3));
    assert_eq!(inv.coeffs[1], (0, 1));
}

// --- Norm for various elements ---

#[test]
fn norm_various_sqrt2() {
    let f = AlgField::from_sqrt(2);
    // norm(√2) = -2 (product: √2 * (-√2) = -2)
    assert_eq!(f.gen().norm(), (-2, 1));
    // norm(3) = 9
    assert_eq!(f.from_rational(3, 1).norm(), (9, 1));
    // norm(1+√2) = 1-2 = -1
    assert_eq!(f.one().add(&f.gen()).norm(), (-1, 1));
    // norm(3+2√2) = 9-8 = 1
    let a = f.from_rational(3, 1).add(&f.gen().scale(2, 1));
    assert_eq!(a.norm(), (1, 1));
}

#[test]
fn norm_sqrt5() {
    let f = AlgField::from_sqrt(5);
    // norm(1+√5) = 1-5 = -4
    assert_eq!(f.one().add(&f.gen()).norm(), (-4, 1));
    // Golden ratio: φ = (1+√5)/2, norm = (1-5)/4 = -1
    let phi = f.from_rational(1, 2).add(&f.gen().scale(1, 2));
    assert_eq!(phi.norm(), (-1, 1));
}

// --- Cube root field (degree 3) ---

#[test]
fn cube_root_2_powers() {
    let f = AlgField::from_int_poly(&[-2, 0, 0, 1]); // x³ - 2
    let a = f.gen(); // ∛2

    // ∛2 · ∛2 = ∛4 = α²
    let a2 = a.mul(&a);
    assert_eq!(a2.coeffs, vec![(0,1), (0,1), (1,1)]);

    // ∛2 · ∛4 = ∛8 = 2
    let a3 = a.mul(&a2);
    assert_eq!(a3.coeffs, vec![(2,1), (0,1), (0,1)]);

    // ∛2⁴ = 2·∛2
    let a4 = a.mul(&a3);
    assert_eq!(a4.coeffs, vec![(0,1), (2,1), (0,1)]);

    // ∛2⁵ = 2·∛4
    let a5 = a.mul(&a4);
    assert_eq!(a5.coeffs, vec![(0,1), (0,1), (2,1)]);

    // ∛2⁶ = 4
    let a6 = a.mul(&a5);
    assert_eq!(a6.coeffs, vec![(4,1), (0,1), (0,1)]);
}

#[test]
fn cube_root_2_inverse() {
    let f = AlgField::from_int_poly(&[-2, 0, 0, 1]);
    let a = f.gen(); // ∛2
    let inv = a.inv().unwrap();
    // ∛2 * (1/∛2) = 1
    let prod = a.mul(&inv);
    assert_eq!(prod.coeffs[0], (1, 1));
    assert_eq!(prod.coeffs[1], (0, 1));
    assert_eq!(prod.coeffs[2], (0, 1));
}

#[test]
fn cube_root_2_inv_sum() {
    let f = AlgField::from_int_poly(&[-2, 0, 0, 1]);
    let a = f.gen();
    // 1/(1 + ∛2) — should exist since x³-2 is irreducible and 1+∛2 ≠ 0
    let b = f.one().add(&a);
    let inv = b.inv().unwrap();
    let prod = b.mul(&inv);
    assert_eq!(prod.coeffs[0], (1, 1), "should be 1, got {:?}", prod.coeffs);
    assert_eq!(prod.coeffs[1], (0, 1));
    assert_eq!(prod.coeffs[2], (0, 1));
}

// --- Quartic field ---

#[test]
fn quartic_field() {
    // Q(α) where α⁴ = 2
    let f = AlgField::from_int_poly(&[-2, 0, 0, 0, 1]); // x⁴ - 2
    let a = f.gen();

    // α⁴ = 2
    let a2 = a.mul(&a);
    let a4 = a2.mul(&a2);
    assert_eq!(a4.coeffs[0], (2, 1));
    assert!(a4.coeffs[1..].iter().all(|(n,_)| *n == 0));

    // α⁸ = 4
    let a8 = a4.mul(&a4);
    assert_eq!(a8.coeffs[0], (4, 1));
}

// --- Cyclotomic: x² + x + 1 (primitive cube root of unity) ---

#[test]
fn cube_root_of_unity() {
    // ω satisfies x² + x + 1 = 0, so ω² = -ω - 1
    let f = AlgField::from_int_poly(&[1, 1, 1]); // x² + x + 1
    let w = f.gen();

    // ω² + ω + 1 = 0
    let w2 = w.mul(&w);
    let sum = w2.add(&w).add(&f.one());
    assert!(sum.is_zero(), "ω²+ω+1 should be 0, got {}", sum);

    // ω³ = 1 (since ω² = -ω-1, ω³ = ω·ω² = ω(-ω-1) = -ω²-ω = (ω+1)-ω = 1)
    let w3 = w.mul(&w2);
    assert_eq!(w3.coeffs[0], (1, 1), "ω³ should be 1");
    assert_eq!(w3.coeffs[1], (0, 1));
}

// --- Edge cases ---

#[test]
fn zero_inverse_fails() {
    let f = AlgField::from_sqrt(2);
    assert!(f.zero().inv().is_none());
}

#[test]
fn one_inverse() {
    let f = AlgField::from_sqrt(2);
    let inv = f.one().inv().unwrap();
    assert_eq!(inv.coeffs[0], (1, 1));
}

#[test]
fn distributive_law() {
    let f = AlgField::from_sqrt(3);
    let a = f.from_rational(2, 3).add(&f.gen().scale(1, 5));
    let b = f.from_rational(-1, 2).add(&f.gen().scale(3, 7));
    let c = f.from_rational(4, 1).add(&f.gen().scale(-2, 3));
    // a*(b+c) = a*b + a*c
    let lhs = a.mul(&b.add(&c));
    let rhs = a.mul(&b).add(&a.mul(&c));
    assert_eq!(lhs, rhs, "distributive law failed");
}

#[test]
fn associative_mul() {
    let f = AlgField::from_sqrt(5);
    let a = f.from_rational(1, 3).add(&f.gen().scale(2, 7));
    let b = f.from_rational(-3, 4).add(&f.gen());
    let c = f.from_rational(5, 2).add(&f.gen().scale(-1, 3));
    // (a*b)*c = a*(b*c)
    let lhs = a.mul(&b).mul(&c);
    let rhs = a.mul(&b.mul(&c));
    assert_eq!(lhs, rhs, "associativity failed");
}
