//! Orthogonal polynomials as a Maxima plugin.
//!
//! Each family is computed from its exact three-term recurrence over rational
//! coefficients (`num::BigRational`, so no integer overflow at any degree),
//! then emitted as a polynomial in the supplied argument and simplified by the
//! host. The argument may be symbolic (`legendre_p(2, x)` -> `(3*x^2-1)/2`) or
//! numeric (`legendre_p(2, 1/2)` -> `-1/8`).
//!
//! Functions: legendre_p, chebyshev_t, chebyshev_u, hermite, laguerre,
//! gen_laguerre, jacobi_p, ultraspherical (Gegenbauer). Parametrised families
//! (gen_laguerre/jacobi_p/ultraspherical) require numeric rational parameters;
//! a symbolic parameter yields the noun form.
//!
//! Degree is capped at MAX_N. Symbolic arguments are exact at any degree up to
//! the cap. Numeric arguments at high degree are limited by the host, which
//! lacks big-rational arithmetic: the value stays correct but may be left
//! partly unsimplified (the coefficients there exceed i64).

use maxima_plugin::{maxima_plugin, meval, Expr, Environment, Operator, guard};
use num::{BigInt, BigRational, One, Zero, ToPrimitive};

/// Largest degree we will expand; beyond this we return the noun form rather
/// than build an unboundedly large expression.
const MAX_N: u32 = 100;

type Q = BigRational;

fn qi(n: i64) -> Q { Q::from_integer(BigInt::from(n)) }
fn qr(n: i64, d: i64) -> Q { Q::new(BigInt::from(n), BigInt::from(d)) }

// ---- argument extraction -------------------------------------------------

/// A non-negative integer degree within the cap.
fn extract_n(e: &Expr) -> Option<u32> {
    let n = match e {
        Expr::Integer(n) if *n >= 0 => *n,
        _ => return None,
    };
    let n: u32 = n.try_into().ok()?;
    if n <= MAX_N { Some(n) } else { None }
}

/// A numeric rational parameter (integer or rational; not float/symbolic).
fn extract_q(e: &Expr) -> Option<Q> {
    match e {
        Expr::Integer(n) => Some(qi(*n)),
        Expr::Rational { num, den } => Some(qr(*num, *den)),
        Expr::BigInt(b) => Some(Q::from_integer((**b).clone())),
        _ => None,
    }
}

// ---- dense polynomial helpers (index = power of x) -----------------------

fn coeff(p: &[Q], i: usize) -> Q {
    p.get(i).cloned().unwrap_or_else(Q::zero)
}
fn pscale(p: &[Q], s: &Q) -> Vec<Q> { p.iter().map(|c| c * s).collect() }
fn pshift(p: &[Q]) -> Vec<Q> {
    let mut v = Vec::with_capacity(p.len() + 1);
    v.push(Q::zero());
    v.extend_from_slice(p);
    v
}
fn padd(a: &[Q], b: &[Q]) -> Vec<Q> {
    (0..a.len().max(b.len())).map(|i| coeff(a, i) + coeff(b, i)).collect()
}
fn psub(a: &[Q], b: &[Q]) -> Vec<Q> {
    (0..a.len().max(b.len())).map(|i| coeff(a, i) - coeff(b, i)).collect()
}

/// Run a three-term recurrence: given P0, P1 and `step(k, P_k, P_{k-1}) -> P_{k+1}`,
/// return the coefficients of P_n.
fn run(n: u32, p0: Vec<Q>, p1: Vec<Q>, step: impl Fn(u32, &[Q], &[Q]) -> Vec<Q>) -> Vec<Q> {
    if n == 0 { return p0; }
    if n == 1 { return p1; }
    let (mut prev, mut cur) = (p0, p1);
    for k in 1..n {
        let next = step(k, &cur, &prev);
        prev = cur;
        cur = next;
    }
    cur
}

// ---- BigRational -> Expr -------------------------------------------------

fn bigint_to_expr(b: &BigInt) -> Expr {
    match b.to_i64() {
        Some(i) => Expr::int(i),
        None => Expr::BigInt(Box::new(b.clone())),
    }
}
fn coeff_to_expr(c: &Q) -> Expr {
    let num = bigint_to_expr(c.numer());
    if c.denom().is_one() { num } else { Expr::div(num, bigint_to_expr(c.denom())) }
}

/// Build `sum_k coeffs[k] * x^k` as an Expr (caller simplifies).
fn poly_to_expr(coeffs: &[Q], x: &Expr) -> Expr {
    let mut terms = Vec::new();
    for (k, c) in coeffs.iter().enumerate() {
        if c.is_zero() { continue; }
        let coeff = coeff_to_expr(c);
        let term = if k == 0 {
            coeff
        } else {
            let power = if k == 1 { x.clone() } else { Expr::pow(x.clone(), Expr::int(k as i64)) };
            Expr::mul(coeff, power)
        };
        terms.push(term);
    }
    match terms.len() {
        0 => Expr::int(0),
        1 => terms.pop().unwrap(),
        _ => Expr::List { op: Operator::MPlus, simplified: false, args: terms },
    }
}

// ---- families ------------------------------------------------------------

fn legendre_p(args: &[Expr], env: &mut Environment) -> Expr {
    guard("legendre_p", args, || {
        let (Some(n), x) = (extract_n(&args[0]), &args[1]) else {
            return Expr::call("legendre_p", args.to_vec());
        };
        // (k+1) P_{k+1} = (2k+1) x P_k - k P_{k-1}
        let cs = run(n, vec![qi(1)], vec![qi(0), qi(1)], |k, pk, pk1| {
            let k = k as i64;
            pscale(
                &psub(&pscale(&pshift(pk), &qi(2 * k + 1)), &pscale(pk1, &qi(k))),
                &qr(1, k + 1),
            )
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn chebyshev_t(args: &[Expr], env: &mut Environment) -> Expr {
    guard("chebyshev_t", args, || {
        let (Some(n), x) = (extract_n(&args[0]), &args[1]) else {
            return Expr::call("chebyshev_t", args.to_vec());
        };
        // T_{k+1} = 2 x T_k - T_{k-1}
        let cs = run(n, vec![qi(1)], vec![qi(0), qi(1)], |_k, pk, pk1| {
            psub(&pscale(&pshift(pk), &qi(2)), pk1)
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn chebyshev_u(args: &[Expr], env: &mut Environment) -> Expr {
    guard("chebyshev_u", args, || {
        let (Some(n), x) = (extract_n(&args[0]), &args[1]) else {
            return Expr::call("chebyshev_u", args.to_vec());
        };
        // U_{k+1} = 2 x U_k - U_{k-1};  U_0 = 1, U_1 = 2x
        let cs = run(n, vec![qi(1)], vec![qi(0), qi(2)], |_k, pk, pk1| {
            psub(&pscale(&pshift(pk), &qi(2)), pk1)
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn hermite(args: &[Expr], env: &mut Environment) -> Expr {
    guard("hermite", args, || {
        let (Some(n), x) = (extract_n(&args[0]), &args[1]) else {
            return Expr::call("hermite", args.to_vec());
        };
        // H_{k+1} = 2 x H_k - 2k H_{k-1};  H_0 = 1, H_1 = 2x
        let cs = run(n, vec![qi(1)], vec![qi(0), qi(2)], |k, pk, pk1| {
            psub(&pscale(&pshift(pk), &qi(2)), &pscale(pk1, &qi(2 * k as i64)))
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn laguerre(args: &[Expr], env: &mut Environment) -> Expr {
    guard("laguerre", args, || {
        let (Some(n), x) = (extract_n(&args[0]), &args[1]) else {
            return Expr::call("laguerre", args.to_vec());
        };
        // (k+1) L_{k+1} = (2k+1 - x) L_k - k L_{k-1};  L_0 = 1, L_1 = 1 - x
        let cs = run(n, vec![qi(1)], vec![qi(1), qi(-1)], |k, pk, pk1| {
            let k = k as i64;
            let term = psub(
                &psub(&pscale(pk, &qi(2 * k + 1)), &pshift(pk)),
                &pscale(pk1, &qi(k)),
            );
            pscale(&term, &qr(1, k + 1))
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn gen_laguerre(args: &[Expr], env: &mut Environment) -> Expr {
    guard("gen_laguerre", args, || {
        let noun = || Expr::call("gen_laguerre", args.to_vec());
        let (Some(n), Some(a), x) = (extract_n(&args[0]), extract_q(&args[1]), &args[2]) else {
            return noun();
        };
        // (k+1) L_{k+1}^a = (2k+1+a - x) L_k^a - (k+a) L_{k-1}^a
        // L_0 = 1, L_1 = 1 + a - x
        let p1 = vec![qi(1) + &a, qi(-1)];
        let a_cl = a.clone();
        let cs = run(n, vec![qi(1)], p1, move |k, pk, pk1| {
            let k = k as i64;
            let term = psub(
                &psub(&pscale(pk, &(qi(2 * k + 1) + &a_cl)), &pshift(pk)),
                &pscale(pk1, &(qi(k) + &a_cl)),
            );
            pscale(&term, &qr(1, k + 1))
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn ultraspherical(args: &[Expr], env: &mut Environment) -> Expr {
    guard("ultraspherical", args, || {
        let noun = || Expr::call("ultraspherical", args.to_vec());
        let (Some(n), Some(a), x) = (extract_n(&args[0]), extract_q(&args[1]), &args[2]) else {
            return noun();
        };
        // (k+1) C_{k+1} = 2(k+a) x C_k - (k+2a-1) C_{k-1};  C_0 = 1, C_1 = 2a x
        let p1 = vec![qi(0), qi(2) * &a];
        let a_cl = a.clone();
        let cs = run(n, vec![qi(1)], p1, move |k, pk, pk1| {
            let k = k as i64;
            let two_k_plus_a = qi(2) * (qi(k) + &a_cl);
            let k_plus_2a_m1 = qi(k - 1) + qi(2) * &a_cl;
            let term = psub(&pscale(&pshift(pk), &two_k_plus_a), &pscale(pk1, &k_plus_2a_m1));
            pscale(&term, &qr(1, k + 1))
        });
        meval(&poly_to_expr(&cs, x), env)
    })
}

fn jacobi_p(args: &[Expr], env: &mut Environment) -> Expr {
    guard("jacobi_p", args, || {
        let noun = || Expr::call("jacobi_p", args.to_vec());
        let (Some(n), Some(a), Some(b), x) =
            (extract_n(&args[0]), extract_q(&args[1]), extract_q(&args[2]), &args[3])
        else {
            return noun();
        };
        // P_0 = 1; P_1 = (a-b)/2 + (a+b+2)/2 x
        let p1 = vec![(&a - &b) / qi(2), (&a + &b + qi(2)) / qi(2)];
        let (a_cl, b_cl) = (a.clone(), b.clone());
        // For m = k+1 >= 2:
        //   c1 P_m = (c2 x + c3) P_{m-1} - c4 P_{m-2}
        let cs = run(n, vec![qi(1)], p1, move |k, pk, pk1| {
            let m = qi(k as i64 + 1);
            let s = &m + &a_cl + &b_cl; // 2m+a+b is below; build pieces
            let two_m_ab = qi(2) * &m + &a_cl + &b_cl;
            let c1 = qi(2) * &m * (&m + &a_cl + &b_cl) * (&two_m_ab - qi(2));
            if c1.is_zero() {
                // Degenerate parameters; bail to noun via empty marker.
                return vec![];
            }
            let c2 = (&two_m_ab - qi(1)) * &two_m_ab * (&two_m_ab - qi(2));
            let c3 = (&two_m_ab - qi(1)) * (&a_cl.clone() * &a_cl - &b_cl.clone() * &b_cl);
            let c4 = qi(2) * (&m + &a_cl - qi(1)) * (&m + &b_cl - qi(1)) * &two_m_ab;
            let _ = &s;
            let num = psub(
                &padd(&pscale(&pshift(pk), &c2), &pscale(pk, &c3)),
                &pscale(pk1, &c4),
            );
            pscale(&num, &(Q::one() / c1))
        });
        if n >= 2 && cs.is_empty() {
            return noun();
        }
        meval(&poly_to_expr(&cs, x), env)
    })
}

maxima_plugin!(register = |env| {
    env.register_native("legendre_p", legendre_p, 2, Some(2));
    env.register_native("chebyshev_t", chebyshev_t, 2, Some(2));
    env.register_native("chebyshev_u", chebyshev_u, 2, Some(2));
    env.register_native("hermite", hermite, 2, Some(2));
    env.register_native("laguerre", laguerre, 2, Some(2));
    env.register_native("gen_laguerre", gen_laguerre, 3, Some(3));
    env.register_native("ultraspherical", ultraspherical, 3, Some(3));
    env.register_native("jacobi_p", jacobi_p, 4, Some(4));
});
