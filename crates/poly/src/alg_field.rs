use std::fmt;

/// An algebraic number field Q(α) where α satisfies minpoly(α) = 0.
#[derive(Debug, Clone)]
pub struct AlgField {
    pub minpoly: Vec<(i64, i64)>,
    pub degree: usize,
    pub name: String,
}

/// An element of Q(α): a₀ + a₁α + ... + a_{d-1}α^{d-1}
#[derive(Debug, Clone)]
pub struct AlgNumber {
    pub coeffs: Vec<(i64, i64)>,
    pub field: AlgField,
}

impl AlgField {
    pub fn from_sqrt(n: i64) -> Self {
        AlgField { minpoly: vec![(-n, 1), (0, 1), (1, 1)], degree: 2, name: format!("sqrt({})", n) }
    }

    pub fn from_int_poly(coeffs: &[i64]) -> Self {
        AlgField { minpoly: coeffs.iter().map(|c| (*c, 1)).collect(), degree: coeffs.len() - 1, name: "α".to_string() }
    }

    pub fn gen(&self) -> AlgNumber {
        let mut coeffs = vec![(0, 1); self.degree];
        if self.degree >= 2 { coeffs[1] = (1, 1); }
        AlgNumber { coeffs, field: self.clone() }
    }

    pub fn from_rational(&self, num: i64, den: i64) -> AlgNumber {
        let mut coeffs = vec![(0, 1); self.degree];
        coeffs[0] = reduce_rat(num, den);
        AlgNumber { coeffs, field: self.clone() }
    }

    pub fn zero(&self) -> AlgNumber { AlgNumber { coeffs: vec![(0, 1); self.degree], field: self.clone() } }
    pub fn one(&self) -> AlgNumber { self.from_rational(1, 1) }
}

impl AlgNumber {
    pub fn is_zero(&self) -> bool { self.coeffs.iter().all(|(n, _)| *n == 0) }

    pub fn add(&self, other: &AlgNumber) -> AlgNumber {
        let d = self.field.degree;
        let coeffs = (0..d).map(|i| rat_add(self.coeffs[i], other.coeffs[i])).collect();
        AlgNumber { coeffs, field: self.field.clone() }
    }

    pub fn sub(&self, other: &AlgNumber) -> AlgNumber {
        let d = self.field.degree;
        let coeffs = (0..d).map(|i| rat_sub(self.coeffs[i], other.coeffs[i])).collect();
        AlgNumber { coeffs, field: self.field.clone() }
    }

    pub fn neg(&self) -> AlgNumber {
        AlgNumber { coeffs: self.coeffs.iter().map(|(n, d)| (-n, *d)).collect(), field: self.field.clone() }
    }

    pub fn scale(&self, num: i64, den: i64) -> AlgNumber {
        let coeffs = self.coeffs.iter().map(|(n, d)| reduce_rat(n * num, d * den)).collect();
        AlgNumber { coeffs, field: self.field.clone() }
    }

    pub fn mul(&self, other: &AlgNumber) -> AlgNumber {
        let d = self.field.degree;
        let mut product = vec![(0i64, 1i64); 2 * d - 1];
        for i in 0..d {
            if self.coeffs[i].0 == 0 { continue; }
            for j in 0..d {
                if other.coeffs[j].0 == 0 { continue; }
                product[i + j] = rat_add(product[i + j], rat_mul(self.coeffs[i], other.coeffs[j]));
            }
        }
        self.reduce_mod_minpoly(&product)
    }

    pub fn inv(&self) -> Option<AlgNumber> {
        if self.is_zero() { return None; }
        let (gcd, s, _) = poly_ext_gcd_rat(&self.coeffs, &self.field.minpoly);
        if gcd.is_empty() { return None; }
        let g0 = gcd[0];
        if g0.0 == 0 { return None; }
        let d = self.field.degree;
        let mut result = vec![(0i64, 1i64); d];
        for (i, c) in s.iter().enumerate() {
            if i < d { result[i] = rat_div(*c, g0)?; }
        }
        Some(AlgNumber { coeffs: result, field: self.field.clone() })
    }

    pub fn div(&self, other: &AlgNumber) -> Option<AlgNumber> {
        Some(self.mul(&other.inv()?))
    }

    /// Norm: product of all conjugates. Only implemented for degree 2.
    /// Panics for degree > 2 — use `norm_or_none` for safe access.
    pub fn norm(&self) -> (i64, i64) {
        self.norm_or_none().expect("norm() not implemented for degree > 2; use norm_or_none()")
    }

    pub fn norm_or_none(&self) -> Option<(i64, i64)> {
        if self.field.degree == 2 {
            let n = rat_neg(self.field.minpoly[0]);
            let (a, b) = (self.coeffs[0], self.coeffs[1]);
            Some(rat_sub(rat_mul(a, a), rat_mul(n, rat_mul(b, b))))
        } else { None }
    }

    fn reduce_mod_minpoly(&self, poly: &[(i64, i64)]) -> AlgNumber {
        let d = self.field.degree;
        let mp = &self.field.minpoly;
        let mut rem = poly.to_vec();
        while rem.len() > d {
            let top = rem.len() - 1;
            if rem[top].0 == 0 { rem.pop(); continue; }
            let lc = mp[d];
            let scale = match rat_div(rem[top], lc) { Some(s) => s, None => break };
            let shift = top - d;
            for (i, mc) in mp.iter().enumerate() {
                if mc.0 != 0 { rem[i + shift] = rat_sub(rem[i + shift], rat_mul(scale, *mc)); }
            }
            rem.pop();
        }
        rem.resize(d, (0, 1));
        for c in &mut rem { *c = reduce_rat(c.0, c.1); }
        AlgNumber { coeffs: rem, field: self.field.clone() }
    }
}

impl PartialEq for AlgNumber {
    fn eq(&self, other: &Self) -> bool { self.sub(other).is_zero() }
}

impl fmt::Display for AlgNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        for (i, (n, d)) in self.coeffs.iter().enumerate() {
            if *n == 0 { continue; }
            let c = if *d == 1 { format!("{}", n) } else { format!("{}/{}", n, d) };
            if i == 0 { parts.push(c); }
            else if i == 1 {
                if *n == 1 && *d == 1 { parts.push(self.field.name.clone()); }
                else { parts.push(format!("{}*{}", c, self.field.name)); }
            } else { parts.push(format!("{}*{}^{}", c, self.field.name, i)); }
        }
        if parts.is_empty() { write!(f, "0") } else { write!(f, "{}", parts.join(" + ")) }
    }
}

fn rat_add(a: (i64, i64), b: (i64, i64)) -> (i64, i64) { reduce_rat(a.0 * b.1 + b.0 * a.1, a.1 * b.1) }
fn rat_sub(a: (i64, i64), b: (i64, i64)) -> (i64, i64) { reduce_rat(a.0 * b.1 - b.0 * a.1, a.1 * b.1) }
fn rat_mul(a: (i64, i64), b: (i64, i64)) -> (i64, i64) { reduce_rat(a.0 * b.0, a.1 * b.1) }
fn rat_div(a: (i64, i64), b: (i64, i64)) -> Option<(i64, i64)> {
    if b.0 == 0 { None } else { Some(reduce_rat(a.0 * b.1, a.1 * b.0)) }
}
fn rat_neg(a: (i64, i64)) -> (i64, i64) { (-a.0, a.1) }

fn reduce_rat(n: i64, d: i64) -> (i64, i64) {
    if d == 0 || n == 0 { return (0, 1); }
    let g = gcd_u(n.unsigned_abs(), d.unsigned_abs()) as i64;
    let (mut rn, mut rd) = (n / g, d / g);
    if rd < 0 { rn = -rn; rd = -rd; }
    (rn, rd)
}

fn gcd_u(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd_u(b, a % b) } }

fn poly_ext_gcd_rat(a: &[(i64,i64)], b: &[(i64,i64)]) -> (Vec<(i64,i64)>, Vec<(i64,i64)>, Vec<(i64,i64)>) {
    let mut old_r = trim(a); let mut r = trim(b);
    let mut old_s = vec![(1,1)]; let mut s: Vec<(i64,i64)> = vec![(0,1)];
    let mut old_t: Vec<(i64,i64)> = vec![(0,1)]; let mut t = vec![(1,1)];
    while !r.iter().all(|(n,_)| *n==0) {
        let (q, rem) = match pdivmod(&old_r, &r) { Some(x) => x, None => break };
        old_r = r; r = rem;
        let ns = psub(&old_s, &pmul(&q, &s)); old_s = s; s = ns;
        let nt = psub(&old_t, &pmul(&q, &t)); old_t = t; t = nt;
    }
    (old_r, old_s, old_t)
}

fn trim(p: &[(i64,i64)]) -> Vec<(i64,i64)> {
    let mut v = p.to_vec();
    while v.len() > 1 && v.last().map(|(n,_)| *n==0).unwrap_or(false) { v.pop(); }
    if v.is_empty() { v.push((0,1)); } v
}
fn pmul(a: &[(i64,i64)], b: &[(i64,i64)]) -> Vec<(i64,i64)> {
    if a.is_empty() || b.is_empty() { return vec![(0,1)]; }
    let mut r = vec![(0i64,1i64); a.len()+b.len()-1];
    for (i,ac) in a.iter().enumerate() { if ac.0==0{continue;} for (j,bc) in b.iter().enumerate() { r[i+j] = rat_add(r[i+j], rat_mul(*ac,*bc)); }}
    trim(&r)
}
fn psub(a: &[(i64,i64)], b: &[(i64,i64)]) -> Vec<(i64,i64)> {
    let l = a.len().max(b.len()); let mut r = vec![(0i64,1i64);l];
    for (i,c) in a.iter().enumerate() { r[i] = *c; }
    for (i,c) in b.iter().enumerate() { r[i] = rat_sub(r[i], *c); }
    trim(&r)
}
fn pdivmod(a: &[(i64,i64)], b: &[(i64,i64)]) -> Option<(Vec<(i64,i64)>, Vec<(i64,i64)>)> {
    let b = trim(b); if b.iter().all(|(n,_)|*n==0) { return None; }
    let mut rem = a.to_vec(); let db = b.len()-1; let lc = *b.last()?;
    if lc.0 == 0 { return None; }
    if rem.len() <= db { return Some((vec![(0,1)], rem)); }
    let mut q = vec![(0i64,1i64); rem.len()-db];
    while rem.len() > db {
        let lr = *rem.last()?; if lr.0==0 { rem.pop(); continue; }
        let c = rat_div(lr, lc)?; let sh = rem.len()-1-db;
        q[sh] = c;
        for (i,bc) in b.iter().enumerate() { rem[i+sh] = rat_sub(rem[i+sh], rat_mul(c, *bc)); }
        rem.pop();
    }
    Some((trim(&q), trim(&rem)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt2_mul() {
        let f = AlgField::from_sqrt(2);
        let s = f.gen();
        let p = s.mul(&s);
        assert_eq!(p.coeffs[0], (2, 1));
        assert_eq!(p.coeffs[1], (0, 1));
    }

    #[test]
    fn sqrt2_inv() {
        let f = AlgField::from_sqrt(2);
        let s = f.gen();
        let inv = s.inv().unwrap();
        let prod = s.mul(&inv);
        assert_eq!(prod.coeffs[0], (1, 1));
        assert_eq!(prod.coeffs[1], (0, 1));
    }

    #[test]
    fn one_plus_sqrt2_inv() {
        let f = AlgField::from_sqrt(2);
        let a = f.one().add(&f.gen());
        let inv = a.inv().unwrap();
        // 1/(1+√2) = -1 + √2
        assert_eq!(inv.coeffs[0], (-1, 1));
        assert_eq!(inv.coeffs[1], (1, 1));
    }

    #[test]
    fn cube_root() {
        let f = AlgField::from_int_poly(&[-2, 0, 0, 1]);
        let a = f.gen();
        let a3 = a.mul(&a).mul(&a);
        assert_eq!(a3.coeffs[0], (2, 1));
    }

    #[test]
    fn norm_sqrt2() {
        let f = AlgField::from_sqrt(2);
        let a = f.one().add(&f.gen());
        assert_eq!(a.norm(), (-1, 1));
    }

    #[test]
    fn display_test() {
        let f = AlgField::from_sqrt(2);
        let a = f.from_rational(3, 1).add(&f.gen().scale(2, 1));
        assert_eq!(a.to_string(), "3 + 2*sqrt(2)");
    }
}
