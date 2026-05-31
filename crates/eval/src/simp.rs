use maxima_core::{Expr, Operator};

/// Coefficient of a like-term during sum collection. Rationals are kept exact
/// so that e.g. (1/2)*x + (-1/2)*x cancels to 0; any contact with a float
/// coefficient degrades to float (preserving prior behavior, no exactness
/// claim for floats).
#[derive(Clone, Copy, PartialEq)]
enum Coef {
    Rat(i64, i64), // num/den, den > 0, reduced
    Flt(f64),
}

impl Coef {
    fn rat(num: i64, den: i64) -> Coef {
        if den == 0 { return Coef::Flt(f64::NAN); }
        let sign = if (num < 0) ^ (den < 0) { -1i64 } else { 1 };
        let (mut a, mut b) = (num.unsigned_abs(), den.unsigned_abs());
        while b != 0 { let t = b; b = a % b; a = t; }
        let g = a.max(1);
        let n = (num.unsigned_abs() / g) as i64 * sign;
        let d = (den.unsigned_abs() / g) as i64;
        Coef::Rat(n, d)
    }

    fn add(self, other: Coef) -> Coef {
        match (self, other) {
            (Coef::Rat(a, b), Coef::Rat(c, d)) => {
                // a/b + c/d = (a*d + c*b)/(b*d)
                let n = a * d + c * b;
                let den = b * d;
                Coef::rat(n, den)
            }
            (x, y) => Coef::Flt(x.to_f64() + y.to_f64()),
        }
    }

    fn to_f64(self) -> f64 {
        match self {
            Coef::Rat(n, d) => n as f64 / d as f64,
            Coef::Flt(f) => f,
        }
    }

    fn is_zero(self) -> bool {
        match self {
            Coef::Rat(n, _) => n == 0,
            Coef::Flt(f) => f == 0.0,
        }
    }

    /// Render this coefficient as a standalone numeric expression.
    fn to_expr(self) -> Expr {
        match self {
            Coef::Rat(n, 1) => Expr::int(n),
            Coef::Rat(n, d) => Expr::Rational { num: n, den: d },
            Coef::Flt(f) => Expr::Float(f),
        }
    }

    /// Build `coef * base`, normalizing the common small cases.
    fn times(self, base: Expr) -> Option<Expr> {
        match self {
            Coef::Rat(0, _) => None,
            Coef::Rat(1, 1) => Some(base),
            Coef::Rat(-1, 1) => Some(Expr::neg(base)),
            Coef::Rat(n, 1) => Some(Expr::mul(Expr::int(n), base)),
            Coef::Rat(n, d) => Some(Expr::mul(Expr::Rational { num: n, den: d }, base)),
            Coef::Flt(f) if f == 0.0 => None,
            Coef::Flt(f) if f == 1.0 => Some(base),
            Coef::Flt(f) if f == -1.0 => Some(Expr::neg(base)),
            Coef::Flt(f) if f == f.floor() && f.abs() < i64::MAX as f64 =>
                Some(Expr::mul(Expr::int(f as i64), base)),
            Coef::Flt(f) => Some(Expr::mul(Expr::Float(f), base)),
        }
    }
}

/// Like `extract_coeff` but returns an exact `Coef` so rational coefficients
/// collect without losing precision.
fn extract_coeff_c(expr: &Expr) -> (Coef, Expr) {
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        if !args.is_empty() {
            let coef = match &args[0] {
                Expr::Integer(n) => Some(Coef::Rat(*n, 1)),
                Expr::Rational { num, den } => Some(Coef::rat(*num, *den)),
                Expr::Float(f) => Some(Coef::Flt(*f)),
                _ => None,
            };
            if let Some(coef) = coef {
                let mut rest_args: Vec<Expr> = args[1..].to_vec();
                rest_args.sort_by(|a, b| expr_sort_key(a).cmp(&expr_sort_key(b)));
                let rest = if rest_args.len() == 1 {
                    rest_args.pop().unwrap()
                } else {
                    Expr::List { op: Operator::MTimes, simplified: true, args: rest_args }
                };
                return (coef, rest);
            }
            // No numeric leading factor: canonicalize the product as the base.
            let mut sorted = args.clone();
            sorted.sort_by(|a, b| expr_sort_key(a).cmp(&expr_sort_key(b)));
            return (Coef::Rat(1, 1), Expr::List { op: Operator::MTimes, simplified: true, args: sorted });
        }
    }
    (Coef::Rat(1, 1), expr.clone())
}

/// Simplify an expression: collect like terms, flatten nested ops, canonical ordering.
pub fn simplify(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MPlus, args, .. } => simplify_plus(args),
        Expr::List { op: Operator::MTimes, args, .. } => simplify_times(args),
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let base = simplify(&args[0]);
            let exp = simplify(&args[1]);
            simplify_power(&base, &exp)
        }
        Expr::List { op: Operator::MAnd, args, .. } => simplify_and(args),
        Expr::List { op: Operator::MOr, args, .. } => simplify_or(args),
        Expr::List { op: Operator::MNot, args, .. } if args.len() == 1 => {
            simplify_not(&simplify(&args[0]))
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| simplify(a)).collect();
            // Simplify Named function calls
            if let Operator::Named(id) = op {
                let fname = maxima_core::resolve(*id);
                match fname.as_str() {
                    "is" if new_args.len() == 1 => {
                        if is_sym(&new_args[0], "true") { return Expr::sym("true"); }
                        if is_sym(&new_args[0], "false") { return Expr::sym("false"); }
                    }
                    "maybe" if new_args.len() == 1 => {
                        if is_sym(&new_args[0], "true") { return Expr::sym("true"); }
                        if is_sym(&new_args[0], "false") { return Expr::sym("false"); }
                    }
                    "sqrt" if new_args.len() == 1 => {
                        match &new_args[0] {
                            Expr::Integer(n) if *n >= 0 => {
                                let root = (*n as f64).sqrt() as i64;
                                if let Some(sq) = root.checked_mul(root) {
                                    if sq == *n { return Expr::int(root); }
                                }
                                let mut k = root;
                                while k > 1 {
                                    if let Some(k2) = k.checked_mul(k) {
                                        if n % k2 == 0 {
                                            let remainder = n / k2;
                                            if remainder == 1 {
                                                return Expr::int(k);
                                            }
                                            return Expr::mul(
                                                Expr::int(k),
                                                Expr::call("sqrt", vec![Expr::int(remainder)]),
                                            );
                                        }
                                    }
                                    k -= 1;
                                }
                            }
                            Expr::Rational { num, den } if *num >= 0 && *den > 0 => {
                                let nr = (*num as f64).sqrt() as i64;
                                let dr = (*den as f64).sqrt() as i64;
                                if nr.checked_mul(nr) == Some(*num) && dr.checked_mul(dr) == Some(*den) {
                                    return simplify(&Expr::Rational { num: nr, den: dr });
                                }
                            }
                            _ => {}
                        }
                    }
                    "abs" if new_args.len() == 1 => {
                        match &new_args[0] {
                            Expr::Integer(n) => return Expr::int(n.abs()),
                            Expr::Float(f) => return Expr::Float(f.abs()),
                            Expr::Rational { num, den } => return Expr::Rational { num: num.abs(), den: den.abs() },
                            // abs(x^2) → x^2 (always non-negative)
                            Expr::List { op: Operator::MExpt, args: pa, .. } if pa.len() == 2 => {
                                if let Expr::Integer(e) = &pa[1] {
                                    if e % 2 == 0 && *e > 0 {
                                        return new_args[0].clone();
                                    }
                                }
                            }
                            // abs(known sum of squares) simplification
                            Expr::List { op: Operator::MPlus, args: terms, .. } => {
                                let all_nonneg = terms.iter().all(|t| {
                                    matches!(t, Expr::Integer(n) if *n >= 0)
                                    || matches!(t, Expr::List { op: Operator::MExpt, args: pa, .. }
                                        if pa.len() == 2 && matches!(&pa[1], Expr::Integer(e) if e % 2 == 0 && *e > 0))
                                });
                                if all_nonneg {
                                    return new_args[0].clone();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

fn simplify_plus(args: &[Expr]) -> Expr {
    // Flatten nested sums and simplify each arg
    let mut terms: Vec<Expr> = Vec::new();
    for arg in args {
        let s = simplify(arg);
        if let Expr::List { op: Operator::MPlus, args: inner, .. } = s {
            terms.extend(inner);
        } else {
            terms.push(s);
        }
    }

    // Check for list + scalar: distribute scalar across list elements
    let mut lists: Vec<(usize, Vec<Expr>)> = Vec::new();
    let mut scalars: Vec<Expr> = Vec::new();
    for (i, term) in terms.iter().enumerate() {
        if let Expr::List { op: Operator::MList, args: items, .. } = term {
            lists.push((i, items.clone()));
        } else {
            scalars.push(term.clone());
        }
    }
    if lists.len() == 1 && !scalars.is_empty() {
        let (_, list_items) = &lists[0];
        let scalar_sum = if scalars.len() == 1 {
            scalars.pop().unwrap()
        } else {
            Expr::List {
                op: Operator::MPlus,
                simplified: false,
                args: scalars,
            }
        };
        let new_items: Vec<Expr> = list_items.iter().map(|item| {
            simplify(&Expr::add(item.clone(), scalar_sum.clone()))
        }).collect();
        return Expr::list(new_items);
    }

    // Separate numeric and symbolic, collect like terms
    let mut num_sum: i64 = 0;
    let mut float_sum: Option<f64> = None;
    let mut rat_sum: Option<(i64, i64)> = None; // (num, den) accumulator
    let mut term_map: Vec<(Expr, Coef)> = Vec::new(); // (base_expr, exact coefficient)

    for term in &terms {
        match term {
            Expr::Integer(n) => {
                if let Some(ref mut f) = float_sum {
                    *f += *n as f64;
                } else if let Some((ref mut rn, ref mut rd)) = rat_sum {
                    *rn += *n * *rd;
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                } else {
                    num_sum += n;
                }
            }
            Expr::Rational { num, den } => {
                if let Some(ref mut f) = float_sum {
                    *f += *num as f64 / *den as f64;
                } else if let Some((ref mut rn, ref mut rd)) = rat_sum {
                    // rn/rd + num/den = (rn*den + num*rd) / (rd*den)
                    *rn = *rn * *den + *num * *rd;
                    *rd *= *den;
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                } else {
                    // Start rational accumulator, fold in num_sum
                    rat_sum = Some((num_sum * *den + *num, *den));
                    let (ref mut rn, ref mut rd) = rat_sum.as_mut().unwrap();
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                    num_sum = 0;
                }
            }
            Expr::Float(f) => {
                let base = if let Some((rn, rd)) = rat_sum.take() {
                    rn as f64 / rd as f64
                } else {
                    float_sum.unwrap_or(num_sum as f64)
                };
                float_sum = Some(base + f);
                num_sum = 0;
            }
            _ => {
                let (coeff, base) = extract_coeff_c(term);
                if let Some(entry) = term_map.iter_mut().find(|(b, _)| *b == base) {
                    entry.1 = entry.1.add(coeff);
                } else {
                    term_map.push((base, coeff));
                }
            }
        }
    }

    // Pythagorean identity: sin(e)^2 + cos(e)^2 → 1 (returns the combined
    // coefficient contribution of matched pairs).
    let pyth_coef = apply_pythagorean(&mut term_map);

    let mut result: Vec<Expr> = Vec::new();

    // Exactly one of float_sum / rat_sum / num_sum carries the constant
    // (the loop folds them together). Combine with the Pythagorean contribution.
    let const_coef = if let Some(f) = float_sum {
        Coef::Flt(f)
    } else if let Some((rn, rd)) = rat_sum {
        Coef::Rat(rn, rd)
    } else {
        Coef::Rat(num_sum, 1)
    }.add(pyth_coef);

    if !const_coef.is_zero() || term_map.is_empty() {
        result.push(const_coef.to_expr());
    }

    // Add collected terms
    for (base, coeff) in term_map {
        if let Some(term) = coeff.times(base) {
            result.push(term);
        }
    }

    if result.is_empty() {
        Expr::int(0)
    } else if result.len() == 1 {
        result.pop().unwrap()
    } else {
        // Sort: numbers first, then symbolic terms by canonical key
        result.sort_by(|a, b| {
            let (ca, ba) = extract_coeff(a);
            let (cb, bb) = extract_coeff(b);
            let ka = expr_sort_key(&ba);
            let kb = expr_sort_key(&bb);
            // Numbers before symbolic
            match (a.is_atom() && matches!(a, Expr::Integer(_) | Expr::Float(_)),
                   b.is_atom() && matches!(b, Expr::Integer(_) | Expr::Float(_))) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            ka.cmp(&kb).then(ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal))
        });
        Expr::List {
            op: Operator::MPlus,
            simplified: true,
            args: result,
        }
    }
}

fn simplify_times(args: &[Expr]) -> Expr {
    // Flatten nested products and simplify each arg
    let mut factors: Vec<Expr> = Vec::new();
    for arg in args {
        let s = simplify(arg);
        if let Expr::List { op: Operator::MTimes, args: inner, .. } = s {
            factors.extend(inner);
        } else {
            factors.push(s);
        }
    }

    // Separate numeric coefficient and symbolic factors, collect like bases
    let mut num_prod: i64 = 1;
    let mut float_prod: Option<f64> = None;
    let mut rat_num: Option<(i64, i64)> = None; // rational accumulator
    let mut base_map: Vec<(Expr, Expr)> = Vec::new(); // (base, exponent)

    for factor in &factors {
        match factor {
            Expr::Integer(0) => return Expr::int(0),
            Expr::Integer(n) => {
                if let Some(ref mut f) = float_prod {
                    *f *= *n as f64;
                } else if let Some((ref mut rn, ref mut rd)) = rat_num {
                    *rn *= n;
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                } else {
                    num_prod *= n;
                }
            }
            Expr::Rational { num, den } => {
                if let Some((ref mut rn, ref mut rd)) = rat_num {
                    *rn *= num;
                    *rd *= den;
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                } else {
                    rat_num = Some((num_prod * num, *den));
                    let (ref mut rn, ref mut rd) = rat_num.as_mut().unwrap();
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()) as i64;
                    *rn /= g;
                    *rd /= g;
                    num_prod = 1;
                }
            }
            Expr::Float(f) => {
                if *f == 0.0 { return Expr::Float(0.0); }
                let base = float_prod.unwrap_or(num_prod as f64);
                float_prod = Some(base * f);
                num_prod = 1;
            }
            Expr::List { op: Operator::MExpt, args: pow_args, .. } if pow_args.len() == 2 => {
                let base = &pow_args[0];
                let exp = &pow_args[1];
                if let Some(entry) = base_map.iter_mut().find(|(b, _)| b == base) {
                    entry.1 = add_exprs(&entry.1, exp);
                } else {
                    base_map.push((base.clone(), exp.clone()));
                }
            }
            _ => {
                if let Some(entry) = base_map.iter_mut().find(|(b, _)| b == factor) {
                    entry.1 = add_exprs(&entry.1, &Expr::int(1));
                } else {
                    base_map.push((factor.clone(), Expr::int(1)));
                }
            }
        }
    }

    let mut result: Vec<Expr> = Vec::new();

    // Add numeric coefficient
    if let Some(f) = float_prod {
        if f == 0.0 { return Expr::Float(0.0); }
        if f != 1.0 || base_map.is_empty() {
            result.push(Expr::Float(f));
        }
    } else if let Some((rn, rd)) = rat_num {
        if rn == 0 { return Expr::int(0); }
        if rd == 1 {
            if rn != 1 || base_map.is_empty() {
                result.push(Expr::int(rn));
            }
        } else {
            result.push(Expr::Rational { num: rn, den: rd });
        }
    } else if num_prod != 1 || base_map.is_empty() {
        if num_prod == 0 { return Expr::int(0); }
        if base_map.is_empty() || num_prod != 1 {
            result.push(Expr::int(num_prod));
        }
    }

    // Distribute scalar * sum: (-1)*(a+b+c) → -a + -b + -c
    if base_map.len() == 1 {
        if let Expr::List { op: Operator::MPlus, args: sum_terms, .. } = &base_map[0].0 {
            if base_map[0].1 == Expr::int(1) {
                let coeff = if let Some(f) = float_prod {
                    f
                } else if let Some((rn, rd)) = rat_num {
                    if rd == 1 { rn as f64 } else { return Expr::mul(Expr::Rational { num: rn, den: rd }, base_map[0].0.clone()); }
                } else {
                    num_prod as f64
                };
                if coeff != 1.0 {
                    let distributed: Vec<Expr> = sum_terms.iter().map(|t| {
                        simplify(&Expr::mul(Expr::int(coeff as i64), t.clone()))
                    }).collect();
                    return simplify(&Expr::List {
                        op: Operator::MPlus,
                        simplified: false,
                        args: distributed,
                    });
                }
            }
        }
    }

    // Sort bases canonically and add
    base_map.sort_by(|(a, _), (b, _)| expr_sort_key(a).cmp(&expr_sort_key(b)));
    for (base, exp) in base_map {
        let exp_simplified = simplify(&exp);
        result.push(simplify_power(&base, &exp_simplified));
    }

    if result.is_empty() {
        Expr::int(1)
    } else if result.len() == 1 {
        result.pop().unwrap()
    } else {
        Expr::List {
            op: Operator::MTimes,
            simplified: true,
            args: result,
        }
    }
}

fn simplify_power(base: &Expr, exp: &Expr) -> Expr {
    match (base, exp) {
        (_, Expr::Integer(0)) => Expr::int(1),
        (_, Expr::Integer(1)) => base.clone(),
        (Expr::Integer(0), _) => Expr::int(0),
        (Expr::Integer(1), _) => Expr::int(1),
        // %i^n: cyclic (1, %i, -1, -%i)
        (Expr::Symbol(id), Expr::Integer(_)) if maxima_core::resolve(*id) == "%i" => {
            if let Some(r) = crate::complex::simplify_i_power(exp) { return r; }
            Expr::pow(base.clone(), exp.clone())
        }
        (Expr::Integer(b), Expr::Integer(e)) if *e >= 2 && *e <= 30 => {
            if let Some(r) = b.checked_pow(*e as u32) {
                Expr::int(r)
            } else {
                let big = num::BigInt::from(*b);
                Expr::BigInt(Box::new(num::pow::Pow::pow(&big, *e as u64)))
            }
        }
        // (n/d)^e for small integer e: fold to an exact rational. Leave it
        // symbolic if the powers would overflow i64.
        (Expr::Rational { num, den }, Expr::Integer(e)) if e.unsigned_abs() >= 2 && e.unsigned_abs() <= 30 => {
            let k = e.unsigned_abs() as u32;
            match (num.checked_pow(k), den.checked_pow(k)) {
                (Some(np), Some(dp)) => {
                    let (mut rn, mut rd) = if *e > 0 { (np, dp) } else { (dp, np) };
                    if rd == 0 { return Expr::pow(base.clone(), exp.clone()); }
                    if rd < 0 { rn = -rn; rd = -rd; }
                    let g = gcd(rn.unsigned_abs(), rd.unsigned_abs()).max(1) as i64;
                    rn /= g; rd /= g;
                    if rd == 1 { Expr::int(rn) } else { Expr::Rational { num: rn, den: rd } }
                }
                _ => Expr::pow(base.clone(), exp.clone()),
            }
        }
        // sqrt(x)^n: sqrt(x)^2 → x, sqrt(x)^(2k) → x^k, sqrt(x)^(2k+1) → x^k*sqrt(x)
        (Expr::List { op: Operator::Named(id), args, .. }, Expr::Integer(n))
            if args.len() == 1 && maxima_core::resolve(*id) == "sqrt" && *n >= 2 =>
        {
            let inner = &args[0];
            let half = n / 2;
            let remainder = n % 2;
            let whole = if half == 1 { inner.clone() } else { simplify_power(inner, &Expr::int(half)) };
            if remainder == 0 {
                return whole;
            }
            return simplify_times(&[whole, Expr::call("sqrt", vec![inner.clone()])]);
        }
        // (x^(1/2))^n → x^(n/2): must be before general (a^b)^c to avoid being swallowed
        (Expr::List { op: Operator::MExpt, args: pa, .. }, Expr::Integer(n))
            if pa.len() == 2 && pa[1] == Expr::Rational { num: 1, den: 2 } && *n >= 2 =>
        {
            let inner = &pa[0];
            let half = n / 2;
            let remainder = n % 2;
            let whole = if half == 1 { inner.clone() } else { simplify_power(inner, &Expr::int(half)) };
            if remainder == 0 {
                return whole;
            }
            return simplify_times(&[whole, Expr::pow(inner.clone(), Expr::Rational { num: 1, den: 2 })]);
        }
        // (a^b)^c => a^(b*c) when both b and c are integer
        (Expr::List { op: Operator::MExpt, args, .. }, Expr::Integer(c))
            if args.len() == 2 =>
        {
            if let Expr::Integer(b) = &args[1] {
                let new_exp = b * c;
                simplify_power(&args[0], &Expr::int(new_exp))
            } else {
                Expr::pow(base.clone(), exp.clone())
            }
        }
        _ => Expr::pow(base.clone(), exp.clone()),
    }
}

/// Canonical sort key for expressions (used for term ordering)
fn expr_sort_key(expr: &Expr) -> String {
    match expr {
        Expr::Integer(n) => format!("0_{:020}", n + 10_000_000_000),
        Expr::Float(f) => format!("0_{:020}", (*f * 1e10) as i64 + 10_000_000_000),
        Expr::Symbol(id) => format!("1_{}", maxima_core::resolve(*id)),
        Expr::List { op, args, .. } => {
            let mut key = format!("2_{}", op);
            for arg in args {
                key.push('_');
                key.push_str(&expr_sort_key(arg));
            }
            key
        }
        _ => format!("9_{}", expr),
    }
}

pub fn gcd_pub(a: u64, b: u64) -> u64 {
    gcd(a, b)
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Extract coefficient and base from a term.
/// E.g. 3*x => (3.0, x), -1*x => (-1.0, x), x => (1.0, x)
fn extract_coeff(expr: &Expr) -> (f64, Expr) {
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        if !args.is_empty() {
            match &args[0] {
                Expr::Integer(n) => {
                    let mut rest_args: Vec<Expr> = args[1..].to_vec();
                    rest_args.sort_by(|a, b| expr_sort_key(a).cmp(&expr_sort_key(b)));
                    let rest = if rest_args.len() == 1 {
                        rest_args.pop().unwrap()
                    } else {
                        Expr::List {
                            op: Operator::MTimes,
                            simplified: true,
                            args: rest_args,
                        }
                    };
                    return (*n as f64, rest);
                }
                Expr::Float(f) => {
                    let mut rest_args: Vec<Expr> = args[1..].to_vec();
                    rest_args.sort_by(|a, b| expr_sort_key(a).cmp(&expr_sort_key(b)));
                    let rest = if rest_args.len() == 1 {
                        rest_args.pop().unwrap()
                    } else {
                        Expr::List {
                            op: Operator::MTimes,
                            simplified: true,
                            args: rest_args,
                        }
                    };
                    return (*f, rest);
                }
                _ => {
                    // No numeric coefficient — sort the product canonically for comparison
                    let mut sorted = args.clone();
                    sorted.sort_by(|a, b| expr_sort_key(a).cmp(&expr_sort_key(b)));
                    let canonical = Expr::List {
                        op: Operator::MTimes,
                        simplified: true,
                        args: sorted,
                    };
                    return (1.0, canonical);
                }
            }
        }
    }
    (1.0, expr.clone())
}

/// Add two expressions (used internally for exponent collection)
fn add_exprs(a: &Expr, b: &Expr) -> Expr {
    match (a, b) {
        (Expr::Integer(x), Expr::Integer(y)) => Expr::int(x + y),
        (Expr::Float(x), Expr::Float(y)) => Expr::Float(x + y),
        (Expr::Integer(x), Expr::Float(y)) | (Expr::Float(y), Expr::Integer(x)) => {
            Expr::Float(*x as f64 + y)
        }
        _ => Expr::add(a.clone(), b.clone()),
    }
}

/// Extract trig function info: returns (func_name, argument) for sin(e)^2 etc.
fn extract_trig_sq(expr: &Expr) -> Option<(&str, &Expr)> {
    // Match: sin(e)^2 or cos(e)^2
    if let Expr::List { op: Operator::MExpt, args, .. } = expr {
        if args.len() == 2 && args[1] == Expr::int(2) {
            if let Expr::List { op: Operator::Named(id), args: fa, .. } = &args[0] {
                if fa.len() == 1 {
                    let name = maxima_core::resolve(*id);
                    if name == "sin" || name == "cos" {
                        return Some((if name == "sin" { "sin" } else { "cos" }, &fa[0]));
                    }
                }
            }
        }
    }
    None
}

fn apply_pythagorean(term_map: &mut Vec<(Expr, Coef)>) -> Coef {
    let mut added = Coef::Rat(0, 1);
    // Collect trig² info: (index, "sin"|"cos", argument, coefficient)
    let trig_info: Vec<(usize, String, Expr, Coef)> = term_map.iter().enumerate()
        .filter_map(|(i, (base, coeff))| {
            extract_trig_sq(base).map(|(name, arg)| (i, name.to_string(), arg.clone(), *coeff))
        })
        .collect();

    // Find matching pairs
    let mut to_remove = Vec::new();
    let mut used = std::collections::HashSet::new();
    for i in 0..trig_info.len() {
        if used.contains(&i) { continue; }
        for j in (i+1)..trig_info.len() {
            if used.contains(&j) { continue; }
            let (idx_i, ref name_i, ref arg_i, coeff_i) = trig_info[i];
            let (idx_j, ref name_j, ref arg_j, coeff_j) = trig_info[j];
            // sin²(e) + cos²(e) with the same coefficient → contributes that
            // coefficient to the constant term.
            if name_i != name_j && *arg_i == *arg_j
                && (coeff_i.to_f64() - coeff_j.to_f64()).abs() < 1e-15 {
                to_remove.push(idx_i);
                to_remove.push(idx_j);
                added = added.add(coeff_i);
                used.insert(i);
                used.insert(j);
                break;
            }
        }
    }

    // Remove matched pairs (in reverse order to preserve indices)
    to_remove.sort_unstable();
    to_remove.dedup();
    for idx in to_remove.into_iter().rev() {
        term_map.remove(idx);
    }
    added
}

fn is_sym(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Symbol(id) if maxima_core::resolve(*id) == name)
}

fn simplify_and(args: &[Expr]) -> Expr {
    let mut result: Vec<Expr> = Vec::new();
    for arg in args {
        let s = simplify(arg);
        if is_sym(&s, "false") { return Expr::sym("false"); }
        if is_sym(&s, "true") { continue; }
        // Flatten nested and
        if let Expr::List { op: Operator::MAnd, args: inner, .. } = &s {
            result.extend(inner.iter().cloned());
        } else {
            result.push(s);
        }
    }
    if result.is_empty() { return Expr::sym("true"); }
    if result.len() == 1 { return result.pop().unwrap(); }
    Expr::List { op: Operator::MAnd, simplified: true, args: result }
}

fn simplify_or(args: &[Expr]) -> Expr {
    let mut result: Vec<Expr> = Vec::new();
    for arg in args {
        let s = simplify(arg);
        if is_sym(&s, "true") { return Expr::sym("true"); }
        if is_sym(&s, "false") { continue; }
        if let Expr::List { op: Operator::MOr, args: inner, .. } = &s {
            result.extend(inner.iter().cloned());
        } else {
            result.push(s);
        }
    }
    if result.is_empty() { return Expr::sym("false"); }
    if result.len() == 1 { return result.pop().unwrap(); }
    Expr::List { op: Operator::MOr, simplified: true, args: result }
}

fn simplify_not(arg: &Expr) -> Expr {
    if is_sym(arg, "true") { return Expr::sym("false"); }
    if is_sym(arg, "false") { return Expr::sym("true"); }
    if let Expr::List { op: Operator::MNot, args, .. } = arg {
        if args.len() == 1 { return args[0].clone(); }
    }
    // Negate comparisons: not(a < b) → a >= b
    if let Expr::List { op, args, .. } = arg {
        let negated_op = match op {
            Operator::MLessThan => Some(Operator::MGreaterEqual),
            Operator::MGreaterThan => Some(Operator::MLessEqual),
            Operator::MLessEqual => Some(Operator::MGreaterThan),
            Operator::MGreaterEqual => Some(Operator::MLessThan),
            _ => None,
        };
        if let Some(neg_op) = negated_op {
            return Expr::List { op: neg_op, simplified: true, args: args.clone() };
        }
    }
    // De Morgan
    if let Expr::List { op: Operator::MAnd, args, .. } = arg {
        let negated: Vec<Expr> = args.iter().map(|a| simplify_not(a)).collect();
        return simplify_or(&negated);
    }
    if let Expr::List { op: Operator::MOr, args, .. } = arg {
        let negated: Vec<Expr> = args.iter().map(|a| simplify_not(a)).collect();
        return simplify_and(&negated);
    }
    Expr::List { op: Operator::MNot, simplified: true, args: vec![arg.clone()] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::Expr;

    fn s(input: &str) -> String {
        let expr = maxima_parser::parse(input);
        let simplified = simplify(&expr);
        simplified.to_string()
    }

    #[test]
    fn collect_like_addition() {
        // x + x => 2*x
        assert_eq!(s("x+x;"), "2*x");
    }

    #[test]
    fn collect_like_addition_with_coeffs() {
        // 2*x + 3*x => 5*x
        assert_eq!(s("2*x+3*x;"), "5*x");
    }

    #[test]
    fn addition_cancel() {
        // x + (-1)*x => 0
        assert_eq!(s("x+(-1)*x;"), "0");
    }

    #[test]
    fn collect_like_multiplication() {
        // x * x => x^2
        assert_eq!(s("x*x;"), "x^2");
    }

    #[test]
    fn collect_powers_in_product() {
        // x^2 * x^3 => x^5
        assert_eq!(s("x^2*x^3;"), "x^5");
    }

    #[test]
    fn numeric_product_collect() {
        // 2 * 3 * x => 6*x
        assert_eq!(s("2*3*x;"), "6*x");
    }

    #[test]
    fn flatten_nested_sum() {
        // (a+b)+c => a+b+c
        let r = s("(a+b)+c;");
        assert!(r.contains("a") && r.contains("b") && r.contains("c"), "got: {}", r);
    }

    #[test]
    fn power_simplify() {
        assert_eq!(s("x^0;"), "1");
        assert_eq!(s("x^1;"), "x");
        assert_eq!(s("2^3;"), "8");
    }

    #[test]
    fn nested_product_flatten() {
        // (2*x)*(3*y) => 6*x*y
        assert_eq!(s("(2*x)*(3*y);"), "6*x*y");
    }

    #[test]
    fn zero_in_product() {
        assert_eq!(s("0*x;"), "0");
    }

    // --- Canonical ordering ---

    #[test]
    fn canonical_product_ordering() {
        // b*a should sort to a*b
        assert_eq!(s("b*a;"), "a*b");
    }

    #[test]
    fn canonical_sum_ordering() {
        // numbers before symbols
        let r = s("x+1;");
        assert_eq!(r, "1+x");
    }

    #[test]
    fn term_cancellation_different_order() {
        // a*b + (-1)*b*a should cancel to 0
        assert_eq!(s("a*b+(-1)*b*a;"), "0");
    }

    // --- Edge cases ---

    #[test]
    fn single_term_sum() {
        assert_eq!(s("0+x;"), "x");
    }

    #[test]
    fn single_factor_product() {
        assert_eq!(s("1*x;"), "x");
    }

    #[test]
    fn double_negative() {
        assert_eq!(s("(-1)*(-1)*x;"), "x");
    }

    #[test]
    fn power_of_power() {
        // (x^2)^3 => x^6
        assert_eq!(s("(x^2)^3;"), "x^6");
    }

    #[test]
    fn zero_sum() {
        assert_eq!(s("0+0;"), "0");
    }

    #[test]
    fn one_product() {
        assert_eq!(s("1*1;"), "1");
    }

    #[test]
    fn nested_flatten_sum() {
        // ((a+b)+(c+d)) should flatten
        let r = s("(a+b)+(c+d);");
        assert!(r.contains("a") && r.contains("b") && r.contains("c") && r.contains("d"),
            "got: {}", r);
    }

    #[test]
    fn nested_flatten_product() {
        // (a*b)*(c*d) should flatten
        let r = s("(a*b)*(c*d);");
        assert!(r.contains("a") && r.contains("b") && r.contains("c") && r.contains("d"),
            "got: {}", r);
    }

    #[test]
    fn collect_three_like_terms() {
        assert_eq!(s("x+x+x;"), "3*x");
    }

    #[test]
    fn collect_three_like_factors() {
        assert_eq!(s("x*x*x;"), "x^3");
    }

    #[test]
    fn mixed_numeric_and_symbolic() {
        let r = s("3+x+2;");
        assert_eq!(r, "5+x");
    }

    #[test]
    fn rational_in_product() {
        // Construct directly with Rational type
        let e = Expr::List {
            op: Operator::MTimes,
            simplified: false,
            args: vec![
                Expr::int(2),
                Expr::Rational { num: 1, den: 4 },
                Expr::sym("x"),
            ],
        };
        let r = simplify(&e).to_string();
        assert_eq!(r, "(1/2)*x");
    }

    #[test]
    fn simplify_passthrough_atom() {
        assert_eq!(simplify(&Expr::int(42)).to_string(), "42");
        assert_eq!(simplify(&Expr::sym("x")).to_string(), "x");
        assert_eq!(simplify(&Expr::Float(3.14)).to_string(), "3.14");
    }

    #[test]
    fn simplify_function_args() {
        // sin(x+x) should simplify inner to sin(2*x)
        let e = Expr::call("sin", vec![Expr::add(Expr::sym("x"), Expr::sym("x"))]);
        let r = simplify(&e);
        assert_eq!(r.to_string(), "sin(2*x)");
    }
}
