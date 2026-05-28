use maxima_core::{Expr, Operator};
use crate::eval::meval;
use crate::env::Environment;
use crate::simp::simplify;

fn set_elements(expr: &Expr) -> Option<&Vec<Expr>> {
    match expr {
        Expr::List { op: Operator::MSet, args, .. } => Some(args),
        Expr::List { op: Operator::MList, args, .. } => Some(args),
        _ => None,
    }
}

fn make_set(mut items: Vec<Expr>) -> Expr {
    items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    items.dedup();
    Expr::set(items)
}

fn list_to_set(items: &[Expr]) -> Expr {
    make_set(items.to_vec())
}

pub(crate) fn eval_set_func(name: &str, args: &[Expr], env: &mut Environment) -> Option<Expr> {
    match name {
        "set" => {
            Some(make_set(args.to_vec()))
        }
        "setify" => {
            if let Some(elems) = args.first().and_then(set_elements) {
                Some(make_set(elems.clone()))
            } else {
                None
            }
        }
        "listify" => {
            if let Some(Expr::List { op: Operator::MSet, args: elems, .. }) = args.first() {
                Some(Expr::list(elems.clone()))
            } else if let Some(Expr::List { op: Operator::MList, .. }) = args.first() {
                Some(args[0].clone())
            } else {
                None
            }
        }
        "union" => {
            if args.len() >= 2 {
                let mut combined = Vec::new();
                for arg in args {
                    if let Some(elems) = set_elements(arg) {
                        combined.extend(elems.iter().cloned());
                    } else {
                        return None;
                    }
                }
                Some(make_set(combined))
            } else { None }
        }
        "intersection" => {
            if args.len() >= 2 {
                if let (Some(a), Some(b)) = (set_elements(&args[0]), set_elements(&args[1])) {
                    let result: Vec<Expr> = a.iter()
                        .filter(|x| b.contains(x))
                        .cloned().collect();
                    let mut out = make_set(result);
                    for arg in &args[2..] {
                        if let Some(c) = set_elements(arg) {
                            if let Some(elems) = set_elements(&out) {
                                let r: Vec<Expr> = elems.iter()
                                    .filter(|x| c.contains(x))
                                    .cloned().collect();
                                out = make_set(r);
                            }
                        } else { return None; }
                    }
                    Some(out)
                } else { None }
            } else { None }
        }
        "setdifference" => {
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (set_elements(&args[0]), set_elements(&args[1])) {
                    let result: Vec<Expr> = a.iter()
                        .filter(|x| !b.contains(x))
                        .cloned().collect();
                    Some(make_set(result))
                } else { None }
            } else { None }
        }
        "symdifference" => {
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (set_elements(&args[0]), set_elements(&args[1])) {
                    let mut result: Vec<Expr> = a.iter()
                        .filter(|x| !b.contains(x))
                        .cloned().collect();
                    result.extend(b.iter().filter(|x| !a.contains(x)).cloned());
                    Some(make_set(result))
                } else { None }
            } else { None }
        }
        "cardinality" => {
            if let Some(elems) = args.first().and_then(set_elements) {
                Some(Expr::int(elems.len() as i64))
            } else { None }
        }
        "elementp" => {
            if args.len() == 2 {
                if let Some(elems) = set_elements(&args[1]) {
                    Some(if elems.contains(&args[0]) { Expr::sym("true") } else { Expr::sym("false") })
                } else { None }
            } else { None }
        }
        "subsetp" => {
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (set_elements(&args[0]), set_elements(&args[1])) {
                    let is_subset = a.iter().all(|x| b.contains(x));
                    Some(if is_subset { Expr::sym("true") } else { Expr::sym("false") })
                } else { None }
            } else { None }
        }
        "disjointp" => {
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (set_elements(&args[0]), set_elements(&args[1])) {
                    let disjoint = !a.iter().any(|x| b.contains(x));
                    Some(if disjoint { Expr::sym("true") } else { Expr::sym("false") })
                } else { None }
            } else { None }
        }
        "powerset" => {
            if let Some(elems) = args.first().and_then(set_elements) {
                let n = elems.len();
                if n > 20 { return None; }
                let mut subsets = Vec::new();
                for mask in 0..(1u32 << n) {
                    let subset: Vec<Expr> = (0..n)
                        .filter(|&i| mask & (1 << i) != 0)
                        .map(|i| elems[i].clone())
                        .collect();
                    subsets.push(make_set(subset));
                }
                Some(make_set(subsets))
            } else { None }
        }
        "subset" => {
            // subset(S, predicate) — filter elements where predicate returns true
            if args.len() == 2 {
                if let Some(elems) = set_elements(&args[0]) {
                    let pred = &args[1];
                    let mut result = Vec::new();
                    for elem in elems {
                        let test = meval(&Expr::List {
                            op: Operator::Named(maxima_core::intern("funapply")),
                            simplified: false,
                            args: vec![pred.clone(), elem.clone()],
                        }, env);
                        if test == Expr::sym("true") {
                            result.push(elem.clone());
                        }
                    }
                    Some(make_set(result))
                } else { None }
            } else { None }
        }
        _ => None,
    }
}
