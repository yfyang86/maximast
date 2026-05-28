use maxima_core::{Expr, Operator, resolve};

pub fn to_f64(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        _ => None,
    }
}

pub fn to_i64(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Integer(n) => Some(*n),
        _ => None,
    }
}

pub fn is_true(expr: &Expr) -> bool {
    match expr {
        Expr::Symbol(id) => resolve(*id) == "true",
        Expr::List { op: Operator::MLessThan | Operator::MGreaterThan
            | Operator::MLessEqual | Operator::MGreaterEqual
            | Operator::MEqual | Operator::MNotEqual, args, .. } if args.len() == 2 => {
            let result = eval_comparison(op_from_expr(expr), &args[0], &args[1]);
            matches!(result, Expr::Symbol(id) if resolve(id) == "true")
        }
        _ => false,
    }
}

fn op_from_expr(expr: &Expr) -> &Operator {
    match expr {
        Expr::List { op, .. } => op,
        _ => &Operator::MEqual,
    }
}

pub fn is_false(expr: &Expr) -> bool {
    matches!(expr, Expr::Symbol(id) if resolve(*id) == "false")
}

pub fn bool_result(b: bool) -> Expr {
    if b { Expr::sym("true") } else { Expr::sym("false") }
}

pub fn eval_comparison(op: &Operator, lhs: &Expr, rhs: &Expr) -> Expr {
    if let (Some(l), Some(r)) = (to_f64(lhs), to_f64(rhs)) {
        let result = match op {
            Operator::MEqual => l == r,
            Operator::MNotEqual => l != r,
            Operator::MLessThan => l < r,
            Operator::MGreaterThan => l > r,
            Operator::MLessEqual => l <= r,
            Operator::MGreaterEqual => l >= r,
            _ => unreachable!(),
        };
        return bool_result(result);
    }
    if matches!(op, Operator::MEqual) && lhs == rhs {
        return Expr::sym("true");
    }
    Expr::List {
        op: *op,
        simplified: false,
        args: vec![lhs.clone(), rhs.clone()],
    }
}

pub fn eval_not(val: &Expr) -> Expr {
    if let Expr::Symbol(id) = val {
        let name = resolve(*id);
        match name.as_str() {
            "true" => return Expr::sym("false"),
            "false" => return Expr::sym("true"),
            _ => {}
        }
    }
    Expr::List {
        op: Operator::MNot,
        simplified: false,
        args: vec![val.clone()],
    }
}

pub fn subst(new: &Expr, old: &Expr, expr: &Expr) -> Expr {
    if expr == old {
        return new.clone();
    }
    match expr {
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| subst(new, old, a)).collect();
            Expr::List {
                op: *op,
                simplified: *simplified,
                args: new_args,
            }
        }
        _ => expr.clone(),
    }
}

pub fn contains_var(expr: &Expr, var: &Expr) -> bool {
    if expr == var { return true; }
    match expr {
        Expr::List { args, .. } => args.iter().any(|a| contains_var(a, var)),
        _ => false,
    }
}

pub fn format_for_print(expr: &Expr) -> String {
    match expr {
        Expr::String(s) => s.to_string(),
        other => other.to_string(),
    }
}

pub fn find_variable(expr: &Expr) -> Option<maxima_core::SymbolId> {
    match expr {
        Expr::Symbol(id) => {
            let name = resolve(*id);
            let reserved = ["%pi", "%e", "%i", "%phi", "true", "false", "done",
                           "inf", "minf", "und", "ind", "infinity"];
            if !reserved.contains(&name.as_str()) {
                Some(*id)
            } else {
                None
            }
        }
        Expr::List { args, .. } => {
            for arg in args {
                if let Some(id) = find_variable(arg) {
                    return Some(id);
                }
            }
            None
        }
        _ => None,
    }
}

pub fn has_free_variable(expr: &Expr) -> bool {
    match expr {
        Expr::Symbol(id) => {
            let name = resolve(*id);
            let reserved = ["%pi", "%e", "%i", "%phi", "true", "false", "done",
                           "inf", "minf", "und", "ind", "infinity"];
            !reserved.contains(&name.as_str())
        }
        Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. }
        | Expr::BigInt(_) | Expr::String(_) => false,
        Expr::List { args, .. } => args.iter().any(has_free_variable),
    }
}

pub fn is_prime(n: i64) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut i = 5i64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return false; }
        i += 6;
    }
    true
}

pub fn gcd_i64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_i64(b, a % b) }
}

pub fn parse_int_in_base(s: &str, base: i64) -> Option<i64> {
    let negative = s.starts_with('-');
    let digits = if negative { &s[1..] } else { s };
    for ch in digits.chars() {
        let digit_val = match ch {
            '0'..='9' => (ch as i64) - ('0' as i64),
            _ => return None,
        };
        if digit_val >= base {
            return None;
        }
    }
    let mut result: i64 = 0;
    for ch in digits.chars() {
        let digit_val = (ch as i64) - ('0' as i64);
        result = result.checked_mul(base)?.checked_add(digit_val)?;
    }
    if negative { Some(-result) } else { Some(result) }
}

pub fn parse_int_in_base_alpha(s: &str, base: i64) -> Option<i64> {
    let s_lower = s.to_lowercase();
    let mut result: i64 = 0;
    for ch in s_lower.chars() {
        let digit_val = match ch {
            '0'..='9' => (ch as i64) - ('0' as i64),
            'a'..='z' => (ch as i64) - ('a' as i64) + 10,
            _ => return None,
        };
        if digit_val >= base {
            return None;
        }
        result = result.checked_mul(base)?.checked_add(digit_val)?;
    }
    Some(result)
}

pub fn int_to_base_string(n: i64, base: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let negative = n < 0;
    let mut val = n.unsigned_abs();
    let base_u = base as u64;
    let mut digits = Vec::new();
    while val > 0 {
        let d = (val % base_u) as u8;
        let ch = if d < 10 {
            (b'0' + d) as char
        } else {
            (b'A' + d - 10) as char
        };
        digits.push(ch);
        val /= base_u;
    }
    digits.reverse();
    let s: String = digits.into_iter().collect();
    let needs_prefix = base > 10 && s.chars().next().is_some_and(|c| c.is_alphabetic());
    if negative {
        if needs_prefix { format!("-0{}", s) } else { format!("-{}", s) }
    } else if needs_prefix {
        format!("0{}", s)
    } else {
        s
    }
}

pub fn expr_to_float(expr: &Expr) -> Expr {
    match expr {
        Expr::Integer(n) => Expr::Float(*n as f64),
        Expr::Rational { num, den } => Expr::Float(*num as f64 / *den as f64),
        Expr::Float(_) => expr.clone(),
        Expr::Symbol(id) => {
            let name = maxima_core::resolve(*id);
            match name.as_str() {
                "%pi" => Expr::Float(std::f64::consts::PI),
                "%e" => Expr::Float(std::f64::consts::E),
                "%phi" => Expr::Float(1.618033988749895),
                _ => expr.clone(),
            }
        }
        Expr::List { op: Operator::MList, args, .. } => {
            Expr::list(args.iter().map(|a| expr_to_float(a)).collect())
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let floated: Vec<Expr> = args.iter().map(|a| expr_to_float(a)).collect();
            if floated.iter().all(|a| matches!(a, Expr::Float(_) | Expr::Integer(_))) {
                let sum: f64 = floated.iter().map(|a| to_f64(a).unwrap_or(0.0)).sum();
                Expr::Float(sum)
            } else {
                Expr::List { op: Operator::MPlus, simplified: false, args: floated }
            }
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let floated: Vec<Expr> = args.iter().map(|a| expr_to_float(a)).collect();
            if floated.iter().all(|a| matches!(a, Expr::Float(_) | Expr::Integer(_))) {
                let prod: f64 = floated.iter().map(|a| to_f64(a).unwrap_or(1.0)).product();
                Expr::Float(prod)
            } else {
                Expr::List { op: Operator::MTimes, simplified: false, args: floated }
            }
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let b = expr_to_float(&args[0]);
            let e = expr_to_float(&args[1]);
            if let (Some(bv), Some(ev)) = (to_f64(&b), to_f64(&e)) {
                Expr::Float(bv.powf(ev))
            } else {
                Expr::pow(b, e)
            }
        }
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let inner = expr_to_float(&args[0]);
            if let Some(v) = to_f64(&inner) {
                let name = maxima_core::resolve(*id);
                let result = match name.as_str() {
                    "sin" => Some(v.sin()),
                    "cos" => Some(v.cos()),
                    "tan" => Some(v.tan()),
                    "exp" => Some(v.exp()),
                    "log" => Some(v.ln()),
                    "sqrt" => Some(v.sqrt()),
                    "asin" => Some(v.asin()),
                    "acos" => Some(v.acos()),
                    "atan" => Some(v.atan()),
                    "sinh" => Some(v.sinh()),
                    "cosh" => Some(v.cosh()),
                    "tanh" => Some(v.tanh()),
                    "abs" => Some(v.abs()),
                    _ => None,
                };
                if let Some(r) = result { return Expr::Float(r); }
            }
            Expr::call(&maxima_core::resolve(*id), vec![inner])
        }
        _ => Expr::call("float", vec![expr.clone()]),
    }
}

pub fn eval_numeric_fold(args: &[Expr], name: &str, f: fn(f64, f64) -> f64) -> Expr {
    let mut result: Option<f64> = None;
    let mut has_symbolic = false;

    for arg in args {
        if let Some(n) = to_f64(arg) {
            result = Some(match result {
                Some(acc) => f(acc, n),
                None => n,
            });
        } else {
            has_symbolic = true;
        }
    }

    if has_symbolic {
        return Expr::call(name, args.to_vec());
    }

    match result {
        Some(r) if r == r.floor() && r.abs() < i64::MAX as f64 => Expr::int(r as i64),
        Some(r) => Expr::Float(r),
        None => Expr::call(name, args.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn to_f64_integer() { assert_eq!(to_f64(&Expr::int(42)), Some(42.0)); }
    #[test] fn to_f64_float() { assert_eq!(to_f64(&Expr::Float(3.14)), Some(3.14)); }
    #[test] fn to_f64_rational() { assert_eq!(to_f64(&Expr::Rational { num: 1, den: 2 }), Some(0.5)); }
    #[test] fn to_f64_symbol() { assert_eq!(to_f64(&Expr::sym("x")), None); }
    #[test] fn to_i64_integer() { assert_eq!(to_i64(&Expr::int(7)), Some(7)); }
    #[test] fn to_i64_float() { assert_eq!(to_i64(&Expr::Float(7.0)), None); }
    #[test] fn is_true_yes() { assert!(is_true(&Expr::sym("true"))); }
    #[test] fn is_true_no() { assert!(!is_true(&Expr::sym("false"))); }
    #[test] fn is_false_yes() { assert!(is_false(&Expr::sym("false"))); }
    #[test] fn is_false_no() { assert!(!is_false(&Expr::int(0))); }
    #[test] fn bool_result_t() { assert_eq!(bool_result(true), Expr::sym("true")); }
    #[test] fn bool_result_f() { assert_eq!(bool_result(false), Expr::sym("false")); }

    #[test] fn compare_eq() {
        assert_eq!(eval_comparison(&Operator::MEqual, &Expr::int(3), &Expr::int(3)), Expr::sym("true"));
    }
    #[test] fn compare_lt() {
        assert_eq!(eval_comparison(&Operator::MLessThan, &Expr::int(1), &Expr::int(2)), Expr::sym("true"));
    }

    #[test] fn not_true() { assert_eq!(eval_not(&Expr::sym("true")), Expr::sym("false")); }
    #[test] fn not_false() { assert_eq!(eval_not(&Expr::sym("false")), Expr::sym("true")); }

    #[test] fn subst_match() { assert_eq!(subst(&Expr::int(5), &Expr::sym("x"), &Expr::sym("x")), Expr::int(5)); }
    #[test] fn subst_no() { assert_eq!(subst(&Expr::int(5), &Expr::sym("x"), &Expr::sym("y")), Expr::sym("y")); }
    #[test] fn subst_nested() {
        let e = Expr::add(Expr::sym("x"), Expr::int(1));
        let r = subst(&Expr::int(3), &Expr::sym("x"), &e);
        assert_eq!(r, Expr::add(Expr::int(3), Expr::int(1)));
    }

    #[test] fn contains_yes() { assert!(contains_var(&Expr::sym("x"), &Expr::sym("x"))); }
    #[test] fn contains_no() { assert!(!contains_var(&Expr::sym("y"), &Expr::sym("x"))); }

    #[test] fn find_var_sym() { assert!(find_variable(&Expr::sym("x")).is_some()); }
    #[test] fn find_var_const() { assert!(find_variable(&Expr::sym("%pi")).is_none()); }
    #[test] fn free_var_yes() { assert!(has_free_variable(&Expr::sym("x"))); }
    #[test] fn free_var_no() { assert!(!has_free_variable(&Expr::int(42))); }

    #[test] fn prime_yes() { assert!(is_prime(7)); assert!(is_prime(97)); }
    #[test] fn prime_no() { assert!(!is_prime(1)); assert!(!is_prime(4)); }

    #[test] fn gcd_ok() { assert_eq!(gcd_i64(12, 8), 4); }
    #[test] fn gcd_co() { assert_eq!(gcd_i64(7, 11), 1); }

    #[test] fn base2_parse() { assert_eq!(parse_int_in_base("101", 2), Some(5)); }
    #[test] fn base_bad() { assert_eq!(parse_int_in_base("89", 8), None); }
    #[test] fn hex_parse() { assert_eq!(parse_int_in_base_alpha("ff", 16), Some(255)); }
    #[test] fn to_b2() { assert_eq!(int_to_base_string(5, 2), "101"); }
    #[test] fn to_b16() { assert_eq!(int_to_base_string(255, 16), "0FF"); }

    #[test] fn float_conv() { assert_eq!(expr_to_float(&Expr::int(3)), Expr::Float(3.0)); }
    #[test] fn fold_max() {
        let r = eval_numeric_fold(&[Expr::int(3), Expr::int(7)], "max", f64::max);
        assert_eq!(r, Expr::int(7));
    }
}
