//! Univariate polynomials in one variable whose coefficients are arbitrary
//! symbolic `Expr`s. Used for `resultant`/`discriminant` when coefficients are
//! not plain integers (e.g. `discriminant(a*x^2+b*x+c, x)`), which the
//! integer-coefficient `maxima_poly` crate cannot represent.

use maxima_core::{Expr, Operator};
use crate::simp::simplify;
use crate::eval::expand;
use crate::helpers::contains_var;

/// Cap the Sylvester matrix size to keep the symbolic determinant tractable
/// (cofactor expansion is factorial in matrix size).
const MAX_SYLVESTER: usize = 8;

/// A polynomial in `var` with symbolic coefficients. `coeffs[i]` multiplies
/// `var^i`; trailing zero coefficients are trimmed.
#[derive(Clone)]
pub struct PolyExpr {
    pub coeffs: Vec<Expr>,
}

impl PolyExpr {
    /// Build from an expression, treating it as a polynomial in `var`.
    /// Returns `None` if the expression is not polynomial in `var`
    /// (e.g. contains `sin(var)`, `1/var`, or `var^symbolic`).
    pub fn from_expr(e: &Expr, var: &Expr) -> Option<PolyExpr> {
        let expanded = expand(e);
        let terms: Vec<Expr> = match &expanded {
            Expr::List { op: Operator::MPlus, args, .. } => args.clone(),
            other => vec![other.clone()],
        };
        let mut coeffs: Vec<Expr> = Vec::new();
        for t in &terms {
            let (pow, c) = term_power_coeff(t, var)?;
            if coeffs.len() <= pow {
                coeffs.resize(pow + 1, Expr::int(0));
            }
            coeffs[pow] = simplify(&Expr::add(coeffs[pow].clone(), c));
        }
        if coeffs.is_empty() {
            coeffs.push(Expr::int(0));
        }
        let mut p = PolyExpr { coeffs };
        p.trim();
        Some(p)
    }

    fn trim(&mut self) {
        while self.coeffs.len() > 1
            && self.coeffs.last() == Some(&Expr::int(0))
        {
            self.coeffs.pop();
        }
    }

    /// Degree, or None for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        if self.coeffs.len() == 1 && self.coeffs[0] == Expr::int(0) {
            None
        } else {
            Some(self.coeffs.len() - 1)
        }
    }

    pub fn leading_coeff(&self) -> Expr {
        self.coeffs.last().cloned().unwrap_or_else(|| Expr::int(0))
    }

    /// Reconstruct an expression `sum_i coeffs[i]*var^i`.
    pub fn to_expr(&self, var: &Expr) -> Expr {
        let mut terms: Vec<Expr> = Vec::new();
        for (i, c) in self.coeffs.iter().enumerate() {
            if *c == Expr::int(0) { continue; }
            let term = if i == 0 {
                c.clone()
            } else if i == 1 {
                Expr::mul(c.clone(), var.clone())
            } else {
                Expr::mul(c.clone(), Expr::pow(var.clone(), Expr::int(i as i64)))
            };
            terms.push(term);
        }
        match terms.len() {
            0 => Expr::int(0),
            1 => simplify(&terms.pop().unwrap()),
            _ => simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }),
        }
    }

    /// Divide by the linear factor (var - z0) via synthetic division,
    /// returning the quotient (remainder assumed zero for exact division).
    pub fn divide_linear(&self, z0: &Expr) -> PolyExpr {
        let n = self.coeffs.len();
        if n <= 1 {
            return PolyExpr { coeffs: vec![Expr::int(0)] };
        }
        let d = n - 1; // degree
        let mut q = vec![Expr::int(0); d];
        q[d - 1] = self.coeffs[d].clone();
        for i in (1..d).rev() {
            q[i - 1] = simplify(&Expr::add(
                self.coeffs[i].clone(),
                Expr::mul(z0.clone(), q[i].clone()),
            ));
        }
        let mut p = PolyExpr { coeffs: q };
        p.trim();
        p
    }

    /// Formal derivative w.r.t. the variable.
    pub fn derivative(&self) -> PolyExpr {
        if self.coeffs.len() <= 1 {
            return PolyExpr { coeffs: vec![Expr::int(0)] };
        }
        let mut d = Vec::with_capacity(self.coeffs.len() - 1);
        for i in 1..self.coeffs.len() {
            d.push(simplify(&Expr::mul(Expr::int(i as i64), self.coeffs[i].clone())));
        }
        let mut p = PolyExpr { coeffs: d };
        p.trim();
        p
    }
}

/// Decompose a single (post-expand) term into (power_of_var, coefficient).
/// Returns None if the term is not a monomial in `var`.
fn term_power_coeff(term: &Expr, var: &Expr) -> Option<(usize, Expr)> {
    if term == var {
        return Some((1, Expr::int(1)));
    }
    if !contains_var(term, var) {
        return Some((0, term.clone()));
    }
    match term {
        // var^n with n a non-negative integer literal
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var => {
            if let Expr::Integer(n) = &args[1] {
                if *n >= 0 { return Some((*n as usize, Expr::int(1))); }
            }
            None
        }
        // product of factors: collect var power, rest is coefficient
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut power = 0usize;
            let mut coeff_factors: Vec<Expr> = Vec::new();
            for f in args {
                if f == var {
                    power += 1;
                } else if let Expr::List { op: Operator::MExpt, args: pa, .. } = f {
                    if pa.len() == 2 && pa[0] == *var {
                        if let Expr::Integer(n) = &pa[1] {
                            if *n >= 0 { power += *n as usize; continue; }
                        }
                        return None; // var^symbolic or negative power
                    }
                    if contains_var(f, var) { return None; }
                    coeff_factors.push(f.clone());
                } else {
                    if contains_var(f, var) { return None; }
                    coeff_factors.push(f.clone());
                }
            }
            let coeff = match coeff_factors.len() {
                0 => Expr::int(1),
                1 => coeff_factors.pop().unwrap(),
                _ => simplify(&Expr::List {
                    op: Operator::MTimes, simplified: false, args: coeff_factors,
                }),
            };
            Some((power, coeff))
        }
        _ => None, // sin(var), 1/var, etc. — not polynomial
    }
}

/// Resultant of two symbolic-coefficient polynomials via the Sylvester matrix
/// determinant. Returns None if degrees are too large or inputs are degenerate.
pub fn resultant(p: &PolyExpr, q: &PolyExpr) -> Option<Expr> {
    let m = p.degree()?;
    let n = q.degree()?;
    if m == 0 && n == 0 {
        return Some(Expr::int(1));
    }
    let size = m + n;
    if size == 0 || size > MAX_SYLVESTER {
        return None;
    }
    // Sylvester matrix: n rows of p's coeffs (shifted), m rows of q's coeffs.
    // Coefficients are placed high-degree-first across each row.
    let mut mat: Vec<Vec<Expr>> = vec![vec![Expr::int(0); size]; size];
    // p has coeffs[m..0] (high to low); place into first n rows
    for i in 0..n {
        for k in 0..=m {
            // coeff of var^(m-k) is p.coeffs[m-k]
            mat[i][i + k] = p.coeffs[m - k].clone();
        }
    }
    for i in 0..m {
        for k in 0..=n {
            mat[n + i][i + k] = q.coeffs[n - k].clone();
        }
    }
    Some(sym_det(&mat))
}

/// Discriminant of a symbolic-coefficient polynomial:
/// disc(p) = (-1)^(d(d-1)/2) * resultant(p, p') / lc(p).
pub fn discriminant(p: &PolyExpr, var: &Expr) -> Option<Expr> {
    let d = p.degree()?;
    if d < 2 {
        return Some(Expr::int(0));
    }
    let dp = p.derivative();
    let res = resultant(p, &dp)?;
    let lc = p.leading_coeff();
    let sign = if (d * (d - 1) / 2) % 2 == 0 { 1 } else { -1 };
    let signed = if sign == -1 { simplify(&Expr::neg(res)) } else { res };
    // Divide by leading coefficient and clean up.
    let result = simplify(&expand(&Expr::div(signed, lc)));
    let _ = var;
    Some(result)
}

/// Symbolic determinant via cofactor expansion (small matrices only).
fn sym_det(mat: &[Vec<Expr>]) -> Expr {
    let n = mat.len();
    match n {
        0 => Expr::int(1),
        1 => mat[0][0].clone(),
        2 => simplify(&Expr::sub(
            Expr::mul(mat[0][0].clone(), mat[1][1].clone()),
            Expr::mul(mat[0][1].clone(), mat[1][0].clone()),
        )),
        _ => {
            let mut result = Expr::int(0);
            for j in 0..n {
                if mat[0][j] == Expr::int(0) {
                    continue; // skip zero pivots (Sylvester matrices are sparse)
                }
                let minor = sym_det(&minor_matrix(mat, 0, j));
                let term = simplify(&Expr::mul(mat[0][j].clone(), minor));
                let signed = if j % 2 == 0 { term } else { simplify(&Expr::neg(term)) };
                result = simplify(&Expr::add(result, signed));
            }
            simplify(&expand(&result))
        }
    }
}

fn minor_matrix(mat: &[Vec<Expr>], row: usize, col: usize) -> Vec<Vec<Expr>> {
    let n = mat.len();
    let mut sub = Vec::with_capacity(n - 1);
    for i in 0..n {
        if i == row { continue; }
        let mut r = Vec::with_capacity(n - 1);
        for j in 0..n {
            if j == col { continue; }
            r.push(mat[i][j].clone());
        }
        sub.push(r);
    }
    sub
}
