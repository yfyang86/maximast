use maxima_core::{Expr, Operator};
use crate::simp::simplify;
use crate::helpers::{contains_var, subst};

pub(crate) fn eval_expr_manip(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "multthru" => {
            if args.len() == 1 {
                Some(multthru(&args[0]))
            } else if args.len() == 2 {
                Some(multthru(&simplify(&Expr::mul(args[0].clone(), args[1].clone()))))
            } else { None }
        }
        "xthru" => {
            if args.len() == 1 { Some(xthru(&args[0])) } else { None }
        }
        "collectterms" => {
            if args.len() >= 2 {
                let vars: Vec<&Expr> = args[1..].iter().collect();
                Some(collectterms(&args[0], &vars))
            } else { None }
        }
        "at" => {
            if args.len() == 2 {
                Some(eval_at(&args[0], &args[1], env))
            } else { None }
        }
        "lfreeof" => {
            if args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: vars, .. } = &args[0] {
                    let free = vars.iter().all(|v| !contains_var(&args[1], v));
                    Some(if free { Expr::sym("true") } else { Expr::sym("false") })
                } else { None }
            } else { None }
        }
        "lopow" => {
            if args.len() == 2 {
                Some(lopow(&args[0], &args[1]))
            } else { None }
        }
        _ => None,
    }
}

fn multthru(expr: &Expr) -> Expr {
    match expr {
        // a * (b + c) → a*b + a*c
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut sum_idx = None;
            for (i, a) in args.iter().enumerate() {
                if matches!(a, Expr::List { op: Operator::MPlus, .. }) {
                    sum_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = sum_idx {
                let sum_terms = if let Expr::List { op: Operator::MPlus, args: terms, .. } = &args[idx] {
                    terms.clone()
                } else { unreachable!() };
                let other_factors: Vec<Expr> = args.iter().enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, a)| a.clone())
                    .collect();
                let coeff = if other_factors.len() == 1 {
                    other_factors[0].clone()
                } else {
                    Expr::List { op: Operator::MTimes, simplified: false, args: other_factors }
                };
                let distributed: Vec<Expr> = sum_terms.iter()
                    .map(|t| simplify(&Expr::mul(coeff.clone(), t.clone())))
                    .collect();
                simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: distributed })
            } else {
                expr.clone()
            }
        }
        // (a + b) / c → a/c + b/c
        Expr::List { op: Operator::MExpt, args: pa, .. }
            if pa.len() == 2 && pa[1] == Expr::int(-1) =>
        {
            expr.clone()
        }
        _ => {
            // Check if it's a fraction with a sum numerator
            if let Some((num, den)) = extract_num_den(expr) {
                if matches!(&num, Expr::List { op: Operator::MPlus, .. }) {
                    if let Expr::List { op: Operator::MPlus, args: terms, .. } = &num {
                        let distributed: Vec<Expr> = terms.iter()
                            .map(|t| simplify(&Expr::div(t.clone(), den.clone())))
                            .collect();
                        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: distributed });
                    }
                }
            }
            expr.clone()
        }
    }
}

fn xthru(expr: &Expr) -> Expr {
    // Put a sum of fractions over a common denominator
    if let Expr::List { op: Operator::MPlus, args, .. } = expr {
        let mut nums = Vec::new();
        let mut dens = Vec::new();
        for a in args {
            let (n, d) = extract_num_den_or_one(a);
            nums.push(n);
            dens.push(d);
        }
        // Common denominator = product of all denominators (simplified)
        if dens.iter().all(|d| *d == Expr::int(1)) {
            return expr.clone();
        }
        let common_den = if dens.len() == 1 {
            dens[0].clone()
        } else {
            simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: dens.clone() })
        };
        // Each numerator: num_i * (common_den / den_i)
        let new_terms: Vec<Expr> = nums.iter().zip(dens.iter()).map(|(n, d)| {
            if *d == Expr::int(1) {
                simplify(&Expr::mul(n.clone(), common_den.clone()))
            } else {
                let cofactor = simplify(&Expr::div(common_den.clone(), d.clone()));
                simplify(&Expr::mul(n.clone(), cofactor))
            }
        }).collect();
        let new_num = simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: new_terms });
        simplify(&Expr::div(new_num, common_den))
    } else {
        expr.clone()
    }
}

fn collectterms(expr: &Expr, vars: &[&Expr]) -> Expr {
    if let Expr::List { op: Operator::MPlus, args: terms, .. } = expr {
        let var = vars[0];
        let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::new();
        let mut remainder = Vec::new();

        for term in terms {
            if let Some((coeff, power)) = extract_var_power(term, var) {
                if let Some(g) = groups.iter_mut().find(|(p, _)| *p == power) {
                    g.1.push(coeff);
                } else {
                    groups.push((power, vec![coeff]));
                }
            } else {
                remainder.push(term.clone());
            }
        }

        let mut result_terms = Vec::new();
        for (power, coeffs) in &groups {
            let coeff_sum = if coeffs.len() == 1 {
                coeffs[0].clone()
            } else {
                simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: coeffs.clone() })
            };
            if *power == Expr::int(0) {
                result_terms.push(coeff_sum);
            } else if *power == Expr::int(1) {
                result_terms.push(simplify(&Expr::mul(coeff_sum, var.clone())));
            } else {
                result_terms.push(simplify(&Expr::mul(coeff_sum, Expr::pow(var.clone(), power.clone()))));
            }
        }
        result_terms.extend(remainder);
        if result_terms.len() == 1 { return result_terms.pop().unwrap(); }
        simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: result_terms })
    } else {
        expr.clone()
    }
}

fn eval_at(expr: &Expr, subs: &Expr, env: &mut crate::env::Environment) -> Expr {
    match subs {
        // at(expr, [x=a, y=b, ...])
        Expr::List { op: Operator::MList, args: eqs, .. } => {
            let mut result = expr.clone();
            for eq in eqs {
                if let Expr::List { op: Operator::MEqual, args: sides, .. } = eq {
                    if sides.len() == 2 {
                        result = subst(&sides[1], &sides[0], &result);
                    }
                }
            }
            crate::eval::meval(&result, env)
        }
        // at(expr, x=a)
        Expr::List { op: Operator::MEqual, args: sides, .. } if sides.len() == 2 => {
            let result = subst(&sides[1], &sides[0], expr);
            crate::eval::meval(&result, env)
        }
        _ => Expr::call("at", vec![expr.clone(), subs.clone()]),
    }
}

fn lopow(expr: &Expr, var: &Expr) -> Expr {
    if !contains_var(expr, var) { return Expr::int(0); }
    match expr {
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut min_pow: Option<i64> = None;
            for a in args {
                let p = term_power(a, var);
                min_pow = Some(min_pow.map_or(p, |m: i64| m.min(p)));
            }
            Expr::int(min_pow.unwrap_or(0))
        }
        _ => Expr::int(term_power(expr, var)),
    }
}

fn term_power(term: &Expr, var: &Expr) -> i64 {
    if term == var { return 1; }
    if !contains_var(term, var) { return 0; }
    match term {
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var => {
            if let Expr::Integer(e) = &args[1] { *e } else { 1 }
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            args.iter().map(|a| term_power(a, var)).sum()
        }
        _ => 0,
    }
}

fn extract_var_power(term: &Expr, var: &Expr) -> Option<(Expr, Expr)> {
    if !contains_var(term, var) { return None; }
    if term == var { return Some((Expr::int(1), Expr::int(1))); }
    match term {
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var => {
            Some((Expr::int(1), args[1].clone()))
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut coeff_parts = Vec::new();
            let mut power = Expr::int(0);
            let mut found_var = false;
            for a in args {
                if !found_var && a == var {
                    power = Expr::int(1);
                    found_var = true;
                } else if !found_var {
                    if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                        if pa.len() == 2 && pa[0] == *var {
                            power = pa[1].clone();
                            found_var = true;
                            continue;
                        }
                    }
                    coeff_parts.push(a.clone());
                } else {
                    coeff_parts.push(a.clone());
                }
            }
            if !found_var { return None; }
            let coeff = if coeff_parts.is_empty() {
                Expr::int(1)
            } else if coeff_parts.len() == 1 {
                coeff_parts.pop().unwrap()
            } else {
                Expr::List { op: Operator::MTimes, simplified: false, args: coeff_parts }
            };
            Some((coeff, power))
        }
        _ => Some((term.clone(), Expr::int(0))),
    }
}

fn extract_num_den(expr: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut num_parts = Vec::new();
        let mut den_parts = Vec::new();
        for a in args {
            if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                if pa.len() == 2 {
                    if let Expr::Integer(e) = &pa[1] {
                        if *e < 0 {
                            den_parts.push(if *e == -1 { pa[0].clone() } else {
                                Expr::pow(pa[0].clone(), Expr::int(-e))
                            });
                            continue;
                        }
                    }
                }
            }
            num_parts.push(a.clone());
        }
        if den_parts.is_empty() { return None; }
        let num = match num_parts.len() {
            0 => Expr::int(1),
            1 => num_parts.pop().unwrap(),
            _ => Expr::List { op: Operator::MTimes, simplified: false, args: num_parts },
        };
        let den = match den_parts.len() {
            1 => den_parts.pop().unwrap(),
            _ => Expr::List { op: Operator::MTimes, simplified: false, args: den_parts },
        };
        Some((num, den))
    } else { None }
}

fn extract_num_den_or_one(expr: &Expr) -> (Expr, Expr) {
    extract_num_den(expr).unwrap_or_else(|| (expr.clone(), Expr::int(1)))
}
