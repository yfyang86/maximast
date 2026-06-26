use maxima_core::Expr;

pub(crate) fn eval_numtheory_func(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "ifactors" => {
            let n = as_posint(args.first()?)?;
            if n <= 1 { return Some(Expr::list(vec![])); }
            let mut factors = Vec::new();
            let mut rem = n;
            let mut d = 2u64;
            while d * d <= rem {
                if rem % d == 0 {
                    let mut e = 0;
                    while rem % d == 0 { rem /= d; e += 1; }
                    factors.push(Expr::list(vec![Expr::int(d as i64), Expr::int(e)]));
                }
                d += 1;
            }
            if rem > 1 {
                factors.push(Expr::list(vec![Expr::int(rem as i64), Expr::int(1)]));
            }
            Some(Expr::list(factors))
        }
        "totient" => {
            let n = as_posint(args.first()?)?;
            if n <= 1 { return Some(Expr::int(1)); }
            let mut result = n;
            let mut rem = n;
            let mut d = 2u64;
            while d * d <= rem {
                if rem % d == 0 {
                    while rem % d == 0 { rem /= d; }
                    result = result / d * (d - 1);
                }
                d += 1;
            }
            if rem > 1 { result = result / rem * (rem - 1); }
            Some(Expr::int(result as i64))
        }
        "divisors" => {
            let n = as_posint(args.first()?)?;
            let mut divs = Vec::new();
            let mut d = 1u64;
            while d * d <= n {
                if n % d == 0 {
                    divs.push(d);
                    if d != n / d { divs.push(n / d); }
                }
                d += 1;
            }
            divs.sort();
            Some(Expr::list(divs.into_iter().map(|d| Expr::int(d as i64)).collect()))
        }
        "next_prime" => {
            let n = if let Expr::Integer(i) = args.first()? { *i } else { return None };
            let mut candidate = if n < 2 { 2 } else { n + 1 };
            while !is_prime_u64(candidate as u64) { candidate += 1; }
            Some(Expr::int(candidate))
        }
        "prev_prime" => {
            let n = if let Expr::Integer(i) = args.first()? { *i } else { return None };
            if n <= 2 { return None; }
            let mut candidate = n - 1;
            while candidate > 1 && !is_prime_u64(candidate as u64) { candidate -= 1; }
            if candidate <= 1 { None } else { Some(Expr::int(candidate)) }
        }
        "power_mod" => {
            if args.len() == 3 {
                let base = if let Expr::Integer(i) = &args[0] { *i } else { return None };
                let exp = if let Expr::Integer(i) = &args[1] { *i } else { return None };
                let modulus = if let Expr::Integer(i) = &args[2] { *i } else { return None };
                if modulus <= 0 { return None; }
                Some(Expr::int(pow_mod(base, exp, modulus)))
            } else { None }
        }
        "inv_mod" => {
            if args.len() == 2 {
                let a = if let Expr::Integer(i) = &args[0] { *i } else { return None };
                let n = if let Expr::Integer(i) = &args[1] { *i } else { return None };
                if n <= 0 { return None; }
                inv_mod(a, n).map(Expr::int)
            } else { None }
        }
        "jacobi" => {
            if args.len() == 2 {
                let a = if let Expr::Integer(i) = &args[0] { *i } else { return None };
                let n = if let Expr::Integer(i) = &args[1] { *i } else { return None };
                if n <= 0 || n % 2 == 0 { return None; }
                Some(Expr::int(jacobi_symbol(a, n)))
            } else { None }
        }
        "chinese" => {
            if args.len() == 2 {
                if let (
                    Expr::List { op: maxima_core::Operator::MList, args: residues, .. },
                    Expr::List { op: maxima_core::Operator::MList, args: moduli, .. },
                ) = (&args[0], &args[1]) {
                    if residues.len() != moduli.len() { return None; }
                    let rs: Vec<i64> = residues.iter().filter_map(|e| {
                        if let Expr::Integer(i) = e { Some(*i) } else { None }
                    }).collect();
                    let ms: Vec<i64> = moduli.iter().filter_map(|e| {
                        if let Expr::Integer(i) = e { Some(*i) } else { None }
                    }).collect();
                    if rs.len() != moduli.len() || ms.len() != moduli.len() { return None; }
                    chinese_remainder(&rs, &ms).map(Expr::int)
                } else { None }
            } else { None }
        }
        "fibonacci" | "fib" => {
            let n = if let Expr::Integer(i) = args.first()? { *i } else { return None };
            if n < 0 { return None; }
            Some(fib_expr(n as u64))
        }
        "lucas" => {
            let n = if let Expr::Integer(i) = args.first()? { *i } else { return None };
            if n < 0 { return None; }
            Some(lucas_expr(n as u64))
        }
        _ => None,
    }
}

fn as_posint(e: &Expr) -> Option<u64> {
    if let Expr::Integer(n) = e { if *n >= 0 { Some(*n as u64) } else { None } } else { None }
}

fn is_prime_u64(n: u64) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return false; }
        i += 6;
    }
    true
}

fn pow_mod(base: i64, mut exp: i64, modulus: i64) -> i64 {
    if modulus == 1 { return 0; }
    let m = modulus as i128;
    let mut result = 1i128;
    let mut b = ((base % modulus + modulus) % modulus) as i128;
    if exp < 0 {
        if let Some(inv) = inv_mod(base, modulus) {
            b = inv as i128;
            exp = -exp;
        } else { return 0; }
    }
    let mut e = exp as u64;
    while e > 0 {
        if e & 1 == 1 { result = result * b % m; }
        b = b * b % m;
        e >>= 1;
    }
    result as i64
}

fn inv_mod(a: i64, n: i64) -> Option<i64> {
    let (mut old_r, mut r) = (a as i128, n as i128);
    let (mut old_s, mut s) = (1i128, 0i128);
    while r != 0 {
        let q = old_r / r;
        let tmp = r; r = old_r - q * r; old_r = tmp;
        let tmp = s; s = old_s - q * s; old_s = tmp;
    }
    if old_r.abs() != 1 { return None; }
    Some(((old_s % n as i128 + n as i128) % n as i128) as i64)
}

fn jacobi_symbol(mut a: i64, mut n: i64) -> i64 {
    if n <= 0 || n % 2 == 0 { return 0; }
    a = ((a % n) + n) % n;
    let mut result = 1i64;
    while a != 0 {
        while a % 2 == 0 {
            a /= 2;
            if n % 8 == 3 || n % 8 == 5 { result = -result; }
        }
        std::mem::swap(&mut a, &mut n);
        if a % 4 == 3 && n % 4 == 3 { result = -result; }
        a %= n;
    }
    if n == 1 { result } else { 0 }
}

fn chinese_remainder(residues: &[i64], moduli: &[i64]) -> Option<i64> {
    let n: i128 = moduli.iter().map(|&m| m as i128).product();
    let mut result = 0i128;
    for i in 0..residues.len() {
        let mi = moduli[i] as i128;
        let ni = n / mi;
        // Find ni^(-1) mod mi via extended GCD
        let inv = {
            let (mut old_r, mut r) = (ni, mi);
            let (mut old_s, mut s) = (1i128, 0i128);
            while r != 0 {
                let q = old_r / r;
                let tmp = r; r = old_r - q * r; old_r = tmp;
                let tmp = s; s = old_s - q * s; old_s = tmp;
            }
            if old_r.abs() != 1 { return None; }
            ((old_s % mi) + mi) % mi
        };
        result += residues[i] as i128 * ni * inv;
    }
    Some((((result % n) + n) % n) as i64)
}

/// Compute the nth Fibonacci number, using BigInt to avoid overflow.
/// Returns Expr::Integer when it fits in i64, else Expr::BigInt.
fn fib_expr(n: u64) -> Expr {
    use num::BigInt;
    use num::ToPrimitive;
    if n <= 1 { return Expr::int(n as i64); }
    let (mut a, mut b) = (BigInt::from(0), BigInt::from(1));
    for _ in 2..=n {
        let c = &a + &b;
        a = b;
        b = c;
    }
    match b.to_i64() {
        Some(v) => Expr::int(v),
        None => Expr::BigInt(Box::new(b)),
    }
}

fn lucas_expr(n: u64) -> Expr {
    use num::BigInt;
    use num::ToPrimitive;
    // L(0)=2, L(1)=1, L(n)=L(n-1)+L(n-2).
    let (mut a, mut b) = (BigInt::from(2), BigInt::from(1));
    if n == 0 { return Expr::int(2); }
    for _ in 2..=n {
        let c = &a + &b;
        a = b;
        b = c;
    }
    match b.to_i64() {
        Some(v) => Expr::int(v),
        None => Expr::BigInt(Box::new(b)),
    }
}
