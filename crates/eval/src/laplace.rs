use maxima_core::{Expr, Operator, resolve};
use crate::simp::simplify;
use crate::helpers::{contains_var, to_i64};

pub(crate) fn eval_laplace(name: &str, args: &[Expr], _env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "laplace" => {
            if args.len() == 3 {
                if let (Expr::Symbol(_t_id), Expr::Symbol(_s_id)) = (&args[1], &args[2]) {
                    return Some(laplace_transform(&args[0], &args[1], &args[2]));
                }
            }
            None
        }
        "ilt" => {
            if args.len() == 3 {
                if let (Expr::Symbol(_s_id), Expr::Symbol(_t_id)) = (&args[1], &args[2]) {
                    return Some(inverse_laplace(&args[0], &args[1], &args[2]));
                }
            }
            None
        }
        _ => None,
    }
}

fn laplace_transform(f: &Expr, t: &Expr, s: &Expr) -> Expr {
    // Linearity: L{a*f + b*g} = a*L{f} + b*L{g}
    if let Expr::List { op: Operator::MPlus, args, .. } = f {
        let terms: Vec<Expr> = args.iter().map(|a| laplace_transform(a, t, s)).collect();
        if terms.iter().any(|r| is_noun_laplace(r)) {
            return Expr::call("laplace", vec![f.clone(), t.clone(), s.clone()]);
        }
        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms });
    }

    // Factor out constants: L{c*f(t)} = c*L{f(t)}
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        let (constant, dependent): (Vec<&Expr>, Vec<&Expr>) =
            args.iter().partition(|a| !contains_var(a, t));
        if !constant.is_empty() && !dependent.is_empty() {
            let c = if constant.len() == 1 { constant[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: constant.into_iter().cloned().collect() }) };
            let inner = if dependent.len() == 1 { dependent[0].clone() }
                else { Expr::List { op: Operator::MTimes, simplified: false, args: dependent.into_iter().cloned().collect() } };
            let lt = laplace_transform(&inner, t, s);
            if !is_noun_laplace(&lt) {
                return simplify(&Expr::mul(c, lt));
            }
        }
    }

    // Table entries
    if let Some(result) = laplace_table(f, t, s) {
        return result;
    }

    // Shift theorem: L{exp(a*t)*f(t)} = F(s-a)
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        for (i, arg) in args.iter().enumerate() {
            if let Some(a) = extract_exp_coeff(arg, t) {
                let rest: Vec<Expr> = args.iter().enumerate()
                    .filter(|(j, _)| *j != i).map(|(_, e)| e.clone()).collect();
                let inner = if rest.len() == 1 { rest[0].clone() }
                    else { Expr::List { op: Operator::MTimes, simplified: false, args: rest } };
                let shifted_s = simplify(&Expr::sub(s.clone(), a));
                let lt = laplace_transform(&inner, t, &shifted_s);
                if !is_noun_laplace(&lt) {
                    return lt;
                }
            }
        }
    }

    Expr::call("laplace", vec![f.clone(), t.clone(), s.clone()])
}

fn laplace_table(f: &Expr, t: &Expr, s: &Expr) -> Option<Expr> {
    // L{1} = 1/s
    if !contains_var(f, t) {
        return Some(simplify(&Expr::mul(f.clone(), Expr::pow(s.clone(), Expr::int(-1)))));
    }

    // L{t} = 1/s^2
    if f == t {
        return Some(simplify(&Expr::pow(s.clone(), Expr::int(-2))));
    }

    // L{t^n} = n!/s^(n+1)
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[0] == *t {
            if let Some(n) = to_i64(&args[1]) {
                if n >= 0 {
                    let fact = factorial(n);
                    return Some(simplify(&Expr::div(
                        Expr::int(fact),
                        Expr::pow(s.clone(), Expr::int(n + 1)))));
                }
            }
        }
    }

    // L{exp(a*t)} = 1/(s-a)
    if let Some(a) = extract_exp_coeff(f, t) {
        return Some(simplify(&Expr::pow(
            Expr::sub(s.clone(), a), Expr::int(-1))));
    }

    // Named functions of t
    if let Expr::List { op: Operator::Named(id), args: fa, .. } = f {
        let fname = resolve(*id);
        if fa.len() == 1 {
            // L{sin(w*t)} = w/(s^2+w^2)
            // L{cos(w*t)} = s/(s^2+w^2)
            // L{sinh(w*t)} = w/(s^2-w^2)
            // L{cosh(w*t)} = s/(s^2-w^2)
            if let Some(w) = extract_linear_coeff(&fa[0], t) {
                let w2 = simplify(&Expr::mul(w.clone(), w.clone()));
                let s2 = Expr::pow(s.clone(), Expr::int(2));
                match fname.as_str() {
                    "sin" => return Some(simplify(&Expr::div(w, Expr::add(s2, w2)))),
                    "cos" => return Some(simplify(&Expr::div(s.clone(), Expr::add(s2, w2)))),
                    "sinh" => return Some(simplify(&Expr::div(w, Expr::sub(s2, w2)))),
                    "cosh" => return Some(simplify(&Expr::div(s.clone(), Expr::sub(s2, w2)))),
                    _ => {}
                }
            }
            // L{exp(a*t)} already handled above
        }
    }

    None
}

fn inverse_laplace(f: &Expr, s: &Expr, t: &Expr) -> Expr {
    // Linearity
    if let Expr::List { op: Operator::MPlus, args, .. } = f {
        let terms: Vec<Expr> = args.iter().map(|a| inverse_laplace(a, s, t)).collect();
        if terms.iter().any(|r| is_noun_ilt(r)) {
            return Expr::call("ilt", vec![f.clone(), s.clone(), t.clone()]);
        }
        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms });
    }

    // Factor out constants
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        let (constant, dependent): (Vec<&Expr>, Vec<&Expr>) =
            args.iter().partition(|a| !contains_var(a, s));
        if !constant.is_empty() && !dependent.is_empty() {
            let c = if constant.len() == 1 { constant[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: constant.into_iter().cloned().collect() }) };
            let inner = if dependent.len() == 1 { dependent[0].clone() }
                else { Expr::List { op: Operator::MTimes, simplified: false, args: dependent.into_iter().cloned().collect() } };
            let ilt = inverse_laplace(&inner, s, t);
            if !is_noun_ilt(&ilt) {
                return simplify(&Expr::mul(c, ilt));
            }
        }
    }

    if let Some(result) = ilt_table(f, s, t) {
        return result;
    }

    Expr::call("ilt", vec![f.clone(), s.clone(), t.clone()])
}

fn ilt_table(f: &Expr, s: &Expr, t: &Expr) -> Option<Expr> {
    // ILT{1/s} = 1
    if *f == Expr::pow(s.clone(), Expr::int(-1)) {
        return Some(Expr::int(1));
    }

    // ILT{1/s^n} = t^(n-1)/(n-1)!
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[0] == *s {
            if let Some(e) = to_i64(&args[1]) {
                if e < 0 {
                    let n = (-e) as i64;
                    return Some(simplify(&Expr::div(
                        Expr::pow(t.clone(), Expr::int(n - 1)),
                        Expr::int(factorial(n - 1)))));
                }
            }
        }
    }

    // ILT{1/(s-a)} = exp(a*t)
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[1] == Expr::int(-1) {
            if let Expr::List { op: Operator::MPlus, args: sum_args, .. } = &args[0] {
                // s - a or s + (-a)
                if sum_args.len() == 2 {
                    let (s_part, a_neg) = if sum_args[0] == *s {
                        (true, &sum_args[1])
                    } else if sum_args[1] == *s {
                        (true, &sum_args[0])
                    } else { (false, &sum_args[0]) };
                    if s_part && !contains_var(a_neg, s) {
                        let a = simplify(&Expr::neg(a_neg.clone()));
                        return Some(Expr::call("exp", vec![simplify(&Expr::mul(a, t.clone()))]));
                    }
                }
            }
        }
    }

    // ILT{s/(s^2+w^2)} = cos(w*t)
    // ILT{w/(s^2+w^2)} = sin(w*t)
    if let Some((num, den)) = extract_ratio(f, s) {
        if let Some(w2) = extract_s2_plus_w2(&den, s) {
            if num == *s {
                let w = simplify(&Expr::call("sqrt", vec![w2]));
                return Some(Expr::call("cos", vec![simplify(&Expr::mul(w, t.clone()))]));
            }
            if !contains_var(&num, s) {
                let w = simplify(&Expr::call("sqrt", vec![w2.clone()]));
                // num should be w, so sin(w*t)
                return Some(Expr::call("sin", vec![simplify(&Expr::mul(w, t.clone()))]));
            }
        }
    }

    None
}

fn extract_exp_coeff(expr: &Expr, t: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if resolve(*id) == "exp" && args.len() == 1 {
            return extract_linear_coeff(&args[0], t);
        }
    }
    None
}

fn extract_linear_coeff(expr: &Expr, var: &Expr) -> Option<Expr> {
    if expr == var { return Some(Expr::int(1)); }
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        if args.len() == 2 {
            if args[0] == *var && !contains_var(&args[1], var) { return Some(args[1].clone()); }
            if args[1] == *var && !contains_var(&args[0], var) { return Some(args[0].clone()); }
        }
    }
    None
}

fn extract_ratio(expr: &Expr, _s: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut num_parts = Vec::new();
        let mut den_parts = Vec::new();
        for a in args {
            if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                if pa.len() == 2 {
                    if let Expr::Integer(e) = &pa[1] {
                        if *e < 0 {
                            den_parts.push(if *e == -1 { pa[0].clone() }
                                else { Expr::pow(pa[0].clone(), Expr::int(-e)) });
                            continue;
                        }
                    }
                }
            }
            num_parts.push(a.clone());
        }
        if den_parts.is_empty() { return None; }
        let num = if num_parts.is_empty() { Expr::int(1) }
            else if num_parts.len() == 1 { num_parts.pop().unwrap() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: num_parts } };
        let den = if den_parts.len() == 1 { den_parts.pop().unwrap() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: den_parts } };
        Some((num, den))
    } else {
        None
    }
}

fn extract_s2_plus_w2(den: &Expr, s: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::MPlus, args, .. } = den {
        if args.len() == 2 {
            let s2 = Expr::pow(s.clone(), Expr::int(2));
            if args[0] == s2 && !contains_var(&args[1], s) { return Some(args[1].clone()); }
            if args[1] == s2 && !contains_var(&args[0], s) { return Some(args[0].clone()); }
        }
    }
    None
}

fn is_noun_laplace(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), .. } if resolve(*id) == "laplace")
}

fn is_noun_ilt(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), .. } if resolve(*id) == "ilt")
}

fn factorial(n: i64) -> i64 {
    (1..=n).product()
}
