use maxima_core::{Expr, Operator, resolve, intern};
use crate::simp::simplify;
use crate::helpers::contains_var;

pub(crate) fn eval_complex_func(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "realpart" => args.first().map(|a| { let (re, _) = complex_decompose(a); simplify(&re) }),
        "imagpart" => args.first().map(|a| { let (_, im) = complex_decompose(a); simplify(&im) }),
        "conjugate" => args.first().map(|a| {
            let (re, im) = complex_decompose(a);
            simplify(&Expr::sub(re, Expr::mul(im, Expr::sym("%i"))))
        }),
        "rectform" => args.first().map(|a| {
            let expanded = crate::eval::expand(a);
            let (re, im) = complex_decompose(&expanded);
            let re_s = simplify(&re);
            let im_s = simplify(&im);
            if im_s == Expr::int(0) { re_s }
            else if re_s == Expr::int(0) { simplify(&Expr::mul(im_s, Expr::sym("%i"))) }
            else { simplify(&Expr::add(re_s, Expr::mul(im_s, Expr::sym("%i")))) }
        }),
        "cabs" => args.first().map(|a| {
            let (re, im) = complex_decompose(a);
            simplify(&Expr::call("sqrt", vec![simplify(&Expr::add(
                Expr::mul(re.clone(), re),
                Expr::mul(im.clone(), im),
            ))]))
        }),
        _ => None,
    }
}

/// Simplify %i^n in the power simplifier.
pub(crate) fn simplify_i_power(exp: &Expr) -> Option<Expr> {
    if let Expr::Integer(n) = exp {
        let r = ((*n % 4) + 4) % 4;
        match r {
            0 => Some(Expr::int(1)),
            1 => Some(Expr::sym("%i")),
            2 => Some(Expr::int(-1)),
            3 => Some(Expr::neg(Expr::sym("%i"))),
            _ => unreachable!(),
        }
    } else {
        None
    }
}

/// Decompose an expression into (real_part, imaginary_part).
/// Assumes %i is the imaginary unit. Recursively applies %i²=-1.
pub(crate) fn complex_decompose(expr: &Expr) -> (Expr, Expr) {
    let i_sym = intern("%i");

    match expr {
        Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. } | Expr::BigInt(_) => {
            (expr.clone(), Expr::int(0))
        }
        Expr::Symbol(id) if *id == i_sym => {
            (Expr::int(0), Expr::int(1))
        }
        Expr::Symbol(_) | Expr::String(_) => {
            (expr.clone(), Expr::int(0))
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut re = Vec::new();
            let mut im = Vec::new();
            for a in args {
                let (r, i) = complex_decompose(a);
                if r != Expr::int(0) { re.push(r); }
                if i != Expr::int(0) { im.push(i); }
            }
            let re_sum = match re.len() {
                0 => Expr::int(0),
                1 => re.pop().unwrap(),
                _ => Expr::List { op: Operator::MPlus, simplified: false, args: re },
            };
            let im_sum = match im.len() {
                0 => Expr::int(0),
                1 => im.pop().unwrap(),
                _ => Expr::List { op: Operator::MPlus, simplified: false, args: im },
            };
            (re_sum, im_sum)
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            // Multiply all factors, tracking (re, im) pairs
            let mut re = Expr::int(1);
            let mut im = Expr::int(0);
            for a in args {
                let (ar, ai) = complex_decompose(a);
                // (re + im*i) * (ar + ai*i) = (re*ar - im*ai) + (re*ai + im*ar)*i
                let new_re = simplify(&Expr::sub(
                    Expr::mul(re.clone(), ar.clone()),
                    Expr::mul(im.clone(), ai.clone()),
                ));
                let new_im = simplify(&Expr::add(
                    Expr::mul(re, ai),
                    Expr::mul(im, ar),
                ));
                re = new_re;
                im = new_im;
            }
            (re, im)
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            // %i^n
            if let Expr::Symbol(id) = &args[0] {
                if *id == i_sym {
                    if let Some(result) = simplify_i_power(&args[1]) {
                        return complex_decompose(&result);
                    }
                }
            }
            // For non-%i bases, treat as real (conservative)
            if !contains_i(expr) {
                (expr.clone(), Expr::int(0))
            } else {
                // Fallback: can't decompose arbitrary complex powers
                (expr.clone(), Expr::int(0))
            }
        }
        _ => {
            if !contains_i(expr) {
                (expr.clone(), Expr::int(0))
            } else {
                (expr.clone(), Expr::int(0))
            }
        }
    }
}

fn contains_i(expr: &Expr) -> bool {
    let i_sym = intern("%i");
    match expr {
        Expr::Symbol(id) => *id == i_sym,
        Expr::List { args, .. } => args.iter().any(|a| contains_i(a)),
        _ => false,
    }
}
