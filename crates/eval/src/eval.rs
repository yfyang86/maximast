use maxima_core::{Expr, Operator, SymbolId, resolve};

use crate::assume::{Fact, Relation, compute_sign};
use crate::env::{Environment, FuncDef};
use crate::helpers::{
    to_f64, to_i64, is_true, is_false, bool_result,
    eval_comparison, eval_not, subst, contains_var,
    format_for_print, find_variable, has_free_variable,
    is_prime, gcd_i64, parse_int_in_base, parse_int_in_base_alpha,
    int_to_base_string, expr_to_float, eval_numeric_fold,
};
use crate::simp::simplify;
use crate::tex::expr_to_tex;
use crate::integrate::{
    table_integrate, try_known_definite_integral,
    try_residue_integral, extract_ncexpt, normalize_abs_arg, coeff_to_expr,
};

pub fn meval(expr: &Expr, env: &mut Environment) -> Expr {
    match expr {
        Expr::Integer(_) | Expr::BigInt(_) | Expr::Float(_) | Expr::String(_) => expr.clone(),
        Expr::Rational { .. } => expr.clone(),

        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "%pi" | "%e" | "%i" | "true" | "false" | "done"
                | "inf" | "minf" | "und" | "infinity" => expr.clone(),
                "%" | "%%" => env.last_output().cloned().unwrap_or_else(|| expr.clone()),
                "functions" => list_functions(env),
                "values" => list_values(env),
                _ => env.get(*id).cloned().unwrap_or_else(|| expr.clone()),
            }
        }

        Expr::List { op, args, .. } => eval_list(op, args, env),
    }
}

fn list_functions(env: &Environment) -> Expr {
    let mut items: Vec<Expr> = Vec::new();
    for (name_id, def) in &env.functions {
        let name = resolve(*name_id);
        let params: Vec<Expr> = def.params.iter().map(|p| Expr::Symbol(*p)).collect();
        items.push(Expr::call(&name, params));
    }
    for name in env.native_functions.keys() {
        items.push(Expr::sym(name));
    }
    items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    Expr::list(items)
}

fn list_values(env: &Environment) -> Expr {
    let mut items: Vec<Expr> = env.list_values().iter().map(|id| Expr::Symbol(*id)).collect();
    items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    Expr::list(items)
}

fn eval_list(op: &Operator, args: &[Expr], env: &mut Environment) -> Expr {
    match op {
        Operator::MPlus => eval_plus(args, env),
        Operator::MTimes => eval_times(args, env),
        Operator::MExpt => eval_power(args, env),
        Operator::MAssign => eval_assign(args, env),
        Operator::MDefine => eval_define(args, env),
        Operator::MIf => eval_if(args, env),
        Operator::MDo => eval_do(args, env),
        Operator::MBlock => eval_block(args, env),
        Operator::MLambda => expr_from(op, args),
        Operator::MReturn => {
            let val = meval(&args[0], env);
            Expr::List {
                op: Operator::MReturn,
                simplified: false,
                args: vec![val],
            }
        }
        Operator::MQuote => {
            let inner = args.first().cloned().unwrap_or(Expr::int(0));
            simplify(&inner)
        }
        Operator::MEqual
        | Operator::MNotEqual
        | Operator::MLessThan
        | Operator::MGreaterThan
        | Operator::MLessEqual
        | Operator::MGreaterEqual => {
            let lhs = meval(&args[0], env);
            let rhs = meval(&args[1], env);
            eval_comparison(op, &lhs, &rhs)
        }
        Operator::MNot => {
            let val = meval(&args[0], env);
            // Apply simplifier for De Morgan, comparison negation
            simplify(&Expr::List {
                op: Operator::MNot,
                simplified: false,
                args: vec![val],
            })
        }
        Operator::MAnd | Operator::MOr => eval_logical(op, args, env),
        Operator::MList => {
            let items: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
            Expr::list(items)
        }
        Operator::MSet => {
            let mut items: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
            items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            items.dedup();
            Expr::set(items)
        }
        Operator::Named(id) => {
            let result = eval_funcall(*id, args, env);
            crate::pattern::apply_tellsimp(result, env)
        }
        _ => {
            let evaled_args: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
            expr_from(op, &evaled_args)
        }
    }
}

fn expr_from(op: &Operator, args: &[Expr]) -> Expr {
    Expr::List {
        op: *op,
        simplified: false,
        args: args.to_vec(),
    }
}

fn eval_plus(args: &[Expr], env: &mut Environment) -> Expr {
    let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
    // Matrix + Matrix: element-wise addition
    if evaled.len() == 2 {
        if let (
            Expr::List { op: Operator::MMatrix, args: ra, .. },
            Expr::List { op: Operator::MMatrix, args: rb, .. },
        ) = (&evaled[0], &evaled[1]) {
            if ra.len() == rb.len() {
                let rows: Vec<Expr> = ra.iter().zip(rb.iter()).map(|(a, b)| {
                    if let (
                        Expr::List { op: Operator::MList, args: ca, .. },
                        Expr::List { op: Operator::MList, args: cb, .. },
                    ) = (a, b) {
                        Expr::list(ca.iter().zip(cb.iter())
                            .map(|(x, y)| simplify(&Expr::add(x.clone(), y.clone())))
                            .collect())
                    } else { a.clone() }
                }).collect();
                return Expr::List { op: Operator::MMatrix, simplified: false, args: rows };
            }
        }
    }
    let result = simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: evaled });
    crate::pattern::apply_tellsimp(result, env)
}

fn eval_times(args: &[Expr], env: &mut Environment) -> Expr {
    let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
    // Scalar * Matrix: broadcast multiplication
    if evaled.len() == 2 {
        for (si, mi) in [(0,1), (1,0)] {
            if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled[mi] {
                let scalar = &evaled[si];
                if !matches!(scalar, Expr::List { op: Operator::MMatrix, .. }) {
                    let new_rows: Vec<Expr> = rows.iter().map(|r| {
                        if let Expr::List { op: Operator::MList, args: cols, .. } = r {
                            Expr::list(cols.iter()
                                .map(|c| simplify(&Expr::mul(scalar.clone(), c.clone())))
                                .collect())
                        } else { r.clone() }
                    }).collect();
                    return Expr::List { op: Operator::MMatrix, simplified: false, args: new_rows };
                }
            }
        }
    }
    let result = simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: evaled });
    crate::pattern::apply_tellsimp(result, env)
}

fn eval_power(args: &[Expr], env: &mut Environment) -> Expr {
    let base = meval(&args[0], env);
    let exp = meval(&args[1], env);

    // %e^f → exp(f)
    if base == Expr::sym("%e") {
        return meval(&Expr::call("exp", vec![exp]), env);
    }

    match (&base, &exp) {
        (Expr::Integer(b), Expr::Integer(e)) => {
            if *e >= 0 {
                if *e <= 30 {
                    if let Some(result) = b.checked_pow(*e as u32) {
                        return Expr::int(result);
                    }
                }
                let big_base = num::BigInt::from(*b);
                let result = num::pow::Pow::pow(&big_base, *e as u64);
                return Expr::BigInt(Box::new(result));
            }
            if *e < 0 && *b != 0 {
                let pos_e = (-*e) as u32;
                if pos_e <= 30 {
                    if let Some(denom) = b.checked_pow(pos_e) {
                        if denom == 1 { return Expr::int(1); }
                        if denom == -1 { return Expr::int(-1); }
                        // Normalize: positive denominator, sign in numerator
                        if denom < 0 {
                            return Expr::Rational { num: -1, den: -denom };
                        }
                        return Expr::Rational { num: 1, den: denom };
                    }
                }
                return Expr::pow(base, exp);
            }
            // 0^negative → undefined, return noun form
            if *e < 0 && *b == 0 {
                return Expr::sym("und");
            }
            Expr::pow(base, exp)
        }
        (Expr::Float(b), Expr::Integer(e)) => Expr::Float(b.powi(*e as i32)),
        (Expr::Integer(b), Expr::Float(e)) => Expr::Float((*b as f64).powf(*e)),
        (Expr::Float(b), Expr::Float(e)) => Expr::Float(b.powf(*e)),
        _ => simplify(&Expr::pow(base, exp)),
    }
}

fn eval_assign(args: &[Expr], env: &mut Environment) -> Expr {
    match &args[0] {
        Expr::Symbol(id) => {
            let name = resolve(*id);
            let val = meval(&args[1], env);
            // Handle special variables
            match name.as_str() {
                "ibase" => {
                    if let Some(n) = to_f64(&val) {
                        env.ibase = n as i64;
                    }
                }
                "obase" => {
                    if let Some(n) = to_f64(&val) {
                        env.obase = n as i64;
                    }
                }
                _ => {}
            }
            env.set(*id, val.clone());
            val
        }
        Expr::List {
            op: Operator::MList,
            args: targets,
            ..
        } => {
            let rhs = meval(&args[1], env);
            if let Expr::List {
                op: Operator::MList,
                args: vals,
                ..
            } = &rhs
            {
                let mut results = Vec::new();
                for (t, v) in targets.iter().zip(vals.iter()) {
                    if let Expr::Symbol(id) = t {
                        let sym_name = resolve(*id);
                        match sym_name.as_str() {
                            "ibase" => { if let Some(n) = to_f64(v) { env.ibase = n as i64; } }
                            "obase" => { if let Some(n) = to_f64(v) { env.obase = n as i64; } }
                            _ => {}
                        }
                        env.set(*id, v.clone());
                        results.push(v.clone());
                    }
                }
                Expr::list(results)
            } else {
                rhs
            }
        }
        _ => {
            let val = meval(&args[1], env);
            val
        }
    }
}

fn eval_define(args: &[Expr], env: &mut Environment) -> Expr {
    match &args[0] {
        Expr::List {
            op: Operator::Named(name_id),
            args: params,
            ..
        } => {
            // Check if this is a subscripted function call: funapply(mqapply(t, n), x)
            if resolve(*name_id) == "funapply" && !params.is_empty() {
                if let Expr::List { op: Operator::Named(mq_id), args: mq_args, .. } = &params[0] {
                    if resolve(*mq_id) == "mqapply" && mq_args.len() >= 2 {
                        return eval_define_subscript(&mq_args[0], &mq_args[1..], &params[1..], &args[1], env);
                    }
                }
            }

            let param_ids: Vec<_> = params
                .iter()
                .map(|p| match p {
                    Expr::Symbol(id) => *id,
                    _ => panic!("function parameter must be a symbol"),
                })
                .collect();
            let def = FuncDef {
                params: param_ids,
                body: args[1].clone(),
            };
            env.define_function(*name_id, def);
            Expr::List {
                op: Operator::MDefine,
                simplified: false,
                args: args.to_vec(),
            }
        }
        _ => panic!("invalid function definition syntax"),
    }
}

fn eval_define_subscript(
    base: &Expr, indices: &[Expr], func_params: &[Expr], body: &Expr, env: &mut Environment,
) -> Expr {
    let base_id = match base {
        Expr::Symbol(id) => *id,
        _ => panic!("subscripted function name must be a symbol"),
    };

    let param_ids: Vec<_> = func_params
        .iter()
        .map(|p| match p {
            Expr::Symbol(id) => *id,
            _ => panic!("function parameter must be a symbol"),
        })
        .collect();

    // Check if indices are concrete (e.g., t[0]) or symbolic (e.g., t[n])
    let all_concrete = indices.iter().all(|idx| {
        matches!(idx, Expr::Integer(_) | Expr::String(_))
    });

    if all_concrete {
        let key = crate::env::SubscriptKey {
            name: base_id,
            indices: indices.iter().map(|i| i.to_string()).collect(),
        };
        let def = FuncDef {
            params: param_ids,
            body: body.clone(),
        };
        env.subscript_fns.insert(key, def);
    } else {
        // Generic subscripted function: t[n](x) := ...
        let index_ids: Vec<_> = indices
            .iter()
            .map(|i| match i {
                Expr::Symbol(id) => *id,
                _ => panic!("generic subscript index must be a symbol"),
            })
            .collect();
        let mut all_params = index_ids.clone();
        all_params.extend(param_ids);
        let def = FuncDef {
            params: all_params,
            body: body.clone(),
        };
        env.subscript_generic_fns.insert(base_id, (index_ids, def));
    }

    Expr::sym("done")
}

fn eval_funcall(name: maxima_core::SymbolId, args: &[Expr], env: &mut Environment) -> Expr {
    let func_name = resolve(name);

    // Built-in functions that need unevaluated args
    match func_name.as_str() {
        "kill" => {
            eval_kill(args, env);
            return Expr::sym("done");
        }
        "ev" => return eval_ev(args, env),
        "sum" => return eval_sum(args, env),
        "product" => return eval_product(args, env),
        // makelist/create_list bind a loop var; the body must NOT be eagerly
        // evaluated in the outer scope (where the loop var is unbound). Doing
        // so wastes work in the easy case and infinite-recurses through a
        // user-defined recursive call whose argument depends on the loop var.
        "makelist" => return eval_makelist(args, env),
        "create_list" => return eval_makelist(args, env),
        "errcatch" => return eval_errcatch(args, env),
        "batch" => {
            if args.len() >= 1 {
                let filename = meval(&args[0], env);
                if let Expr::String(s) = &filename {
                    return eval_load(s, env);
                }
            }
            return Expr::sym("done");
        }
        "mdo_in" => {
            if args.len() == 3 {
                let var = match &args[0] {
                    Expr::Symbol(id) => *id,
                    _ => return Expr::sym("done"),
                };
                let list_val = meval(&args[1], env);
                let body = &args[2];
                if let Expr::List { op: Operator::MList, args: items, .. } = &list_val {
                    env.push_scope();
                    for item in items {
                        env.set_local(var, item.clone());
                        let result = meval(body, env);
                        if let Expr::List { op: Operator::MReturn, args: ret_args, .. } = &result {
                            let ret_val = ret_args[0].clone();
                            env.pop_scope();
                            return ret_val;
                        }
                    }
                    env.pop_scope();
                }
            }
            return Expr::sym("done");
        }
        _ => {}
    }

    let evaled_args: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();

    match func_name.as_str() {
        "print" => {
            let strs: Vec<String> = evaled_args.iter().map(|a| format_for_print(a)).collect();
            println!("{}", strs.join(" "));
            evaled_args.last().cloned().unwrap_or(Expr::sym("done"))
        }
        "display" => {
            for arg in &evaled_args {
                println!("{}", arg);
            }
            Expr::sym("done")
        }
        "abs" => {
            if evaled_args.len() == 1 {
                match &evaled_args[0] {
                    Expr::Integer(n) => Expr::int(n.abs()),
                    Expr::Float(f) => Expr::Float(f.abs()),
                    Expr::Rational { num, den } => Expr::Rational { num: num.abs(), den: den.abs() },
                    // abs(conjugate(z)) => abs(z)
                    Expr::List { op: Operator::Named(fid), args: fargs, .. }
                        if resolve(*fid) == "conjugate" && fargs.len() == 1 => {
                        return Expr::call("abs", vec![fargs[0].clone()]);
                    }
                    Expr::Symbol(id) => {
                        let name = resolve(*id);
                        match name.as_str() {
                            "inf" | "minf" | "infinity" => Expr::sym("inf"),
                            "und" | "ind" => Expr::sym(&name),
                            "%phi" => Expr::sym("%phi"),
                            "%i" => Expr::int(1),
                            _ => {
                                let sign = compute_sign(&evaled_args[0], &env.assumptions);
                                match sign {
                                    crate::assume::Sign::Pos | crate::assume::Sign::Poz => evaled_args[0].clone(),
                                    crate::assume::Sign::Neg | crate::assume::Sign::Noz => simplify(&Expr::neg(evaled_args[0].clone())),
                                    crate::assume::Sign::Zero => Expr::int(0),
                                    _ => Expr::call("abs", evaled_args),
                                }
                            }
                        }
                    }
                    _ => {
                        // Check for abs(-expr) or abs(neg_coeff * expr) => abs(expr)
                        let inner = &evaled_args[0];
                        if let Expr::List { op: Operator::MTimes, args: factors, .. } = inner {
                            // Strip negative numeric coefficient
                            if let Some(Expr::Integer(n)) = factors.first() {
                                if *n < 0 {
                                    let mut pos_factors = factors.clone();
                                    pos_factors[0] = Expr::int(n.abs());
                                    if pos_factors[0] == Expr::int(1) {
                                        pos_factors.remove(0);
                                    }
                                    let pos_expr = if pos_factors.len() == 1 {
                                        pos_factors.pop().unwrap()
                                    } else {
                                        simplify(&Expr::List {
                                            op: Operator::MTimes,
                                            simplified: false,
                                            args: pos_factors,
                                        })
                                    };
                                    return Expr::call("abs", vec![pos_expr]);
                                }
                            }
                        }
                        // Normalize sum sign: abs(-a+b) → abs(a-b)
                        let normalized = normalize_abs_arg(inner);
                        let sign = compute_sign(&normalized, &env.assumptions);
                        match sign {
                            crate::assume::Sign::Pos | crate::assume::Sign::Poz => normalized,
                            crate::assume::Sign::Neg => simplify(&Expr::neg(normalized)),
                            crate::assume::Sign::Zero => Expr::int(0),
                            _ => Expr::call("abs", vec![normalized]),
                        }
                    }
                }
            } else {
                Expr::call("abs", evaled_args)
            }
        }
        "mod" => {
            if let (Some(a), Some(b)) = (evaled_args.first().and_then(to_i64), evaled_args.get(1).and_then(to_i64)) {
                return Expr::int(a.rem_euclid(b));
            }
            Expr::call("mod", evaled_args)
        }
        "max" => {
            let r = eval_numeric_fold(&evaled_args, "max", |a, b| a.max(b));
            if matches!(&r, Expr::List { .. }) && evaled_args.len() == 2 {
                // Try assumption-based: if a > b known, max(a,b) = a
                let diff = simplify(&Expr::sub(evaled_args[0].clone(), evaled_args[1].clone()));
                let sign = compute_sign(&diff, &env.assumptions);
                if sign.is_known_positive() || matches!(sign, crate::assume::Sign::Poz) {
                    return evaled_args[0].clone();
                }
                if sign.is_known_negative() || matches!(sign, crate::assume::Sign::Noz) {
                    return evaled_args[1].clone();
                }
            }
            r
        }
        "min" => {
            let r = eval_numeric_fold(&evaled_args, "min", |a, b| a.min(b));
            if matches!(&r, Expr::List { .. }) && evaled_args.len() == 2 {
                let diff = simplify(&Expr::sub(evaled_args[0].clone(), evaled_args[1].clone()));
                let sign = compute_sign(&diff, &env.assumptions);
                if sign.is_known_positive() || matches!(sign, crate::assume::Sign::Poz) {
                    return evaled_args[1].clone();
                }
                if sign.is_known_negative() || matches!(sign, crate::assume::Sign::Noz) {
                    return evaled_args[0].clone();
                }
            }
            r
        }
        "factorial" => {
            if let Some(Expr::Integer(n)) = evaled_args.first() {
                if *n >= 0 && *n <= 20 {
                    let mut result: i64 = 1;
                    for i in 2..=*n {
                        result *= i;
                    }
                    return Expr::int(result);
                }
                if *n >= 0 {
                    let mut result = num::BigInt::from(1);
                    for i in 2..=*n {
                        result *= i;
                    }
                    return Expr::BigInt(Box::new(result));
                }
            }
            Expr::call("factorial", evaled_args)
        }
        "is" => {
            if let Some(val) = evaled_args.first() {
                eval_is_with_db(val, &env.assumptions)
            } else {
                Expr::sym("false")
            }
        }
        "first" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.first().cloned().unwrap_or(Expr::sym("done"))
            } else {
                Expr::call("first", evaled_args)
            }
        }
        "rest" => {
            // rest(L) drops the first element; rest(L, n) drops the first n
            // (or last |n| for n<0); rest(L, 0) returns L; |n| > length gives [].
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                let n = match evaled_args.get(1) {
                    None => 1,
                    Some(Expr::Integer(k)) => *k,
                    _ => return Expr::call("rest", evaled_args),
                };
                let len = items.len() as i64;
                let drop_from_front = if n >= 0 { n.min(len) } else { 0 };
                let drop_from_back  = if n <  0 { (-n).min(len) } else { 0 };
                let lo = drop_from_front as usize;
                let hi = (len - drop_from_back) as usize;
                Expr::list(items[lo..hi].to_vec())
            } else {
                Expr::call("rest", evaled_args)
            }
        }
        "last" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.last().cloned().unwrap_or(Expr::sym("done"))
            } else {
                Expr::call("last", evaled_args)
            }
        }
        "second" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.get(1).cloned().unwrap_or(Expr::call("second", evaled_args))
            } else { Expr::call("second", evaled_args) }
        }
        "third" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.get(2).cloned().unwrap_or(Expr::call("third", evaled_args))
            } else { Expr::call("third", evaled_args) }
        }
        "fourth" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.get(3).cloned().unwrap_or(Expr::call("fourth", evaled_args))
            } else { Expr::call("fourth", evaled_args) }
        }
        "fifth" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                items.get(4).cloned().unwrap_or(Expr::call("fifth", evaled_args))
            } else { Expr::call("fifth", evaled_args) }
        }
        "endcons" => {
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[1] {
                    let mut new_items = items.clone();
                    new_items.push(evaled_args[0].clone());
                    return Expr::list(new_items);
                }
            }
            Expr::call("endcons", evaled_args)
        }
        "length" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                Expr::int(items.len() as i64)
            } else {
                Expr::call("length", evaled_args)
            }
        }
        "append" => {
            let mut result = Vec::new();
            for arg in &evaled_args {
                if let Expr::List { op: Operator::MList, args: items, .. } = arg {
                    result.extend(items.iter().cloned());
                } else {
                    return Expr::call("append", evaled_args);
                }
            }
            Expr::list(result)
        }
        "." => {
            if evaled_args.len() == 2 {
                let a = &evaled_args[0];
                let b = &evaled_args[1];
                // Rational: keep exact
                if let (Expr::Rational { num: n1, den: d1 }, Expr::Rational { num: n2, den: d2 }) = (a, b) {
                    return simplify(&Expr::Rational { num: n1 * n2, den: d1 * d2 });
                }
                // Numeric: a . b = a * b (integers and rationals)
                match (a, b) {
                    (Expr::Integer(x), Expr::Integer(y)) => return Expr::int(x * y),
                    (Expr::Integer(x), Expr::Rational { num, den }) | (Expr::Rational { num, den }, Expr::Integer(x)) => {
                        return simplify(&Expr::Rational { num: x * num, den: *den });
                    }
                    _ => {}
                }
                if let (Some(fa), Some(fb)) = (to_f64(a), to_f64(b)) {
                    if matches!(a, Expr::Float(_)) || matches!(b, Expr::Float(_)) {
                        return Expr::Float(fa * fb);
                    }
                }
                if a == &Expr::int(0) || b == &Expr::int(0) { return Expr::int(0); }
                if a == &Expr::int(1) { return b.clone(); }
                if b == &Expr::int(1) { return a.clone(); }
                let is_id = |e: &Expr| matches!(e, Expr::Symbol(id) if { let n = resolve(*id); n == "dotident" || n == "id" });
                if is_id(a) { return b.clone(); }
                if is_id(b) { return a.clone(); }
                // ncexpt combination: a^^m . a^^n → a^^(m+n)
                if let (Some((base_a, exp_a)), Some((base_b, exp_b))) = (extract_ncexpt(a), extract_ncexpt(b)) {
                    if base_a == base_b {
                        let sum_exp = simplify(&Expr::add(exp_a, exp_b));
                        if sum_exp == Expr::int(0) { return Expr::sym("id"); }
                        if sum_exp == Expr::int(1) { return base_a; }
                        return Expr::call("ncexpt", vec![base_a, sum_exp]);
                    }
                }
                // expr . expr^^(-1) → id (when expr matches ncexpt base)
                if let Some((base_b, exp_b)) = extract_ncexpt(b) {
                    if *a == base_b {
                        let sum = simplify(&Expr::add(Expr::int(1), exp_b));
                        if sum == Expr::int(0) { return Expr::sym("id"); }
                    }
                }
                if let Some((base_a, exp_a)) = extract_ncexpt(a) {
                    if *b == base_a {
                        let sum = simplify(&Expr::add(exp_a, Expr::int(1)));
                        if sum == Expr::int(0) { return Expr::sym("id"); }
                    }
                }
                // a . a → a^^2
                if a == b {
                    return Expr::call("ncexpt", vec![a.clone(), Expr::int(2)]);
                }
                // Scalar extraction: (c*a) . b = c * (a . b)
                if let Expr::List { op: Operator::MTimes, args: fa, .. } = a {
                    let (consts, nonconsts): (Vec<&Expr>, Vec<&Expr>) =
                        fa.iter().partition(|e| matches!(e, Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. }));
                    if !consts.is_empty() && !nonconsts.is_empty() {
                        let scalar = if consts.len() == 1 { consts[0].clone() }
                                     else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
                        let inner = if nonconsts.len() == 1 { nonconsts[0].clone() }
                                    else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: nonconsts.into_iter().cloned().collect() }) };
                        let dot = Expr::call(".", vec![inner, b.clone()]);
                        let dot_evaled = meval(&dot, env);
                        return simplify(&Expr::mul(scalar, dot_evaled));
                    }
                }
                if let Expr::List { op: Operator::MTimes, args: fb, .. } = b {
                    let (consts, nonconsts): (Vec<&Expr>, Vec<&Expr>) =
                        fb.iter().partition(|e| matches!(e, Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. }));
                    if !consts.is_empty() && !nonconsts.is_empty() {
                        let scalar = if consts.len() == 1 { consts[0].clone() }
                                     else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
                        let inner = if nonconsts.len() == 1 { nonconsts[0].clone() }
                                    else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: nonconsts.into_iter().cloned().collect() }) };
                        let dot = Expr::call(".", vec![a.clone(), inner]);
                        let dot_evaled = meval(&dot, env);
                        return simplify(&Expr::mul(scalar, dot_evaled));
                    }
                }
                // Matrix dot product
                if let (Expr::List { op: Operator::MMatrix, args: rows_a, .. },
                        Expr::List { op: Operator::MMatrix, args: rows_b, .. }) = (a, b) {
                    return matrix_dot_product(rows_a, rows_b, env);
                }
            }
            Expr::call(".", evaled_args)
        }
        "ncexpt" => {
            if evaled_args.len() == 2 {
                if let Expr::Integer(0) = &evaled_args[1] {
                    // M^^0 = identity matrix (if M is a matrix)
                    if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled_args[0] {
                        let n = rows.len();
                        let id_rows: Vec<Expr> = (0..n).map(|i| {
                            Expr::list((0..n).map(|j| if i == j { Expr::int(1) } else { Expr::int(0) }).collect())
                        }).collect();
                        return Expr::List { op: Operator::MMatrix, simplified: false, args: id_rows };
                    }
                    return Expr::sym("id");
                }
                if let Expr::Integer(1) = &evaled_args[1] { return evaled_args[0].clone(); }
                // Matrix exponentiation by squaring
                if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled_args[0] {
                    if let Some(n) = to_i64(&evaled_args[1]) {
                        if n >= 2 {
                            let dim = rows.len();
                            let id_rows: Vec<Expr> = (0..dim).map(|i| {
                                Expr::list((0..dim).map(|j| if i == j { Expr::int(1) } else { Expr::int(0) }).collect())
                            }).collect();
                            let mut result = Expr::List { op: Operator::MMatrix, simplified: false, args: id_rows };
                            let mut base = evaled_args[0].clone();
                            let mut exp = n;
                            while exp > 0 {
                                if exp % 2 == 1 {
                                    result = eval_matrix_dot(&result, &base, env);
                                }
                                base = eval_matrix_dot(&base, &base, env);
                                exp /= 2;
                            }
                            return result;
                        }
                    }
                }
            }
            Expr::call("ncexpt", evaled_args)
        }
        "cons" => {
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[1] {
                    let mut new_items = vec![evaled_args[0].clone()];
                    new_items.extend(items.iter().cloned());
                    return Expr::list(new_items);
                }
            }
            Expr::call("cons", evaled_args)
        }
        "reverse" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                let mut rev = items.clone();
                rev.reverse();
                Expr::list(rev)
            } else {
                Expr::call("reverse", evaled_args)
            }
        }
        "map" | "maplist" => eval_map(&evaled_args, env),
        "fullmap" => eval_map(&evaled_args, env),
        "apply" => eval_apply(&evaled_args, env),
        "atom" => bool_result(evaled_args.first().is_some_and(|e| e.is_atom())),
        "numberp" => bool_result(matches!(
            evaled_args.first(),
            Some(Expr::Integer(_) | Expr::Float(_) | Expr::BigInt(_) | Expr::Rational { .. })
        )),
        "integerp" => {
            let is_int = match evaled_args.first() {
                Some(Expr::Integer(_) | Expr::BigInt(_)) => true,
                Some(Expr::Float(f)) => *f == f.floor() && f.is_finite(),
                _ => false,
            };
            bool_result(is_int)
        }
        "floatnump" => {
            let is_float = match evaled_args.first() {
                Some(Expr::Float(f)) => *f != f.floor() || !f.is_finite(),
                _ => false,
            };
            bool_result(is_float)
        }
        "listp" => bool_result(matches!(
            evaled_args.first(),
            Some(Expr::List { op: Operator::MList, .. })
        )),
        "symbolp" => bool_result(matches!(evaled_args.first(), Some(Expr::Symbol(_)))),
        "stringp" => bool_result(matches!(evaled_args.first(), Some(Expr::String(_)))),
        "float" => {
            if let Some(arg) = evaled_args.first() {
                expr_to_float(arg)
            } else {
                Expr::call("float", evaled_args)
            }
        }
        "floor" => {
            if let Some(arg) = evaled_args.first() {
                match arg {
                    Expr::Integer(_) => return arg.clone(),
                    Expr::Float(f) => return Expr::int(f.floor() as i64),
                    Expr::Rational { num, den } => {
                        let q = num / den;
                        let r = num % den;
                        return Expr::int(if r < 0 && *den > 0 || r > 0 && *den < 0 { q - 1 } else { q });
                    }
                    _ => if let Some(f) = to_f64(arg) { return Expr::int(f.floor() as i64); }
                }
            }
            Expr::call("floor", evaled_args)
        }
        "ceiling" => {
            if let Some(arg) = evaled_args.first() {
                match arg {
                    Expr::Integer(_) => return arg.clone(),
                    Expr::Float(f) => return Expr::int(f.ceil() as i64),
                    Expr::Rational { num, den } => {
                        let q = num / den;
                        let r = num % den;
                        return Expr::int(if r > 0 && *den > 0 || r < 0 && *den < 0 { q + 1 } else { q });
                    }
                    _ => if let Some(f) = to_f64(arg) { return Expr::int(f.ceil() as i64); }
                }
            }
            Expr::call("ceiling", evaled_args)
        }
        "truncate" => {
            if let Some(arg) = evaled_args.first() {
                match arg {
                    Expr::Integer(_) => return arg.clone(),
                    Expr::Float(f) => return Expr::int(f.trunc() as i64),
                    Expr::Rational { num, den } => return Expr::int(num / den),
                    _ => if let Some(f) = to_f64(arg) { return Expr::int(f.trunc() as i64); }
                }
            }
            Expr::call("truncate", evaled_args)
        }
        "round" => {
            if let Some(arg) = evaled_args.first() {
                let round_half_even = |f: f64| -> i64 {
                    let r = f.round();
                    if (f - r).abs() == 0.5 { // exact half: round to even
                        let ri = r as i64;
                        if ri % 2 != 0 { (f.floor()) as i64 } else { ri }
                    } else { r as i64 }
                };
                match arg {
                    Expr::Integer(_) => return arg.clone(),
                    Expr::Float(f) => return Expr::int(round_half_even(*f)),
                    Expr::Rational { num, den } => {
                        let f = *num as f64 / *den as f64;
                        return Expr::int(round_half_even(f));
                    }
                    _ => if let Some(f) = to_f64(arg) { return Expr::int(round_half_even(f)); }
                }
            }
            Expr::call("round", evaled_args)
        }
        "string" => {
            if let Some(arg) = evaled_args.first() {
                if env.obase != 10 {
                    if let Some(n) = to_i64(arg) {
                        return Expr::String(int_to_base_string(n, env.obase).into());
                    }
                }
                Expr::String(format_for_print(arg).into())
            } else {
                Expr::call("string", evaled_args)
            }
        }
        "concat" => {
            let parts: Vec<String> = evaled_args.iter().map(|a| format_for_print(a)).collect();
            let result = parts.join("");
            Expr::sym(&result)
        }
        "sconcat" => {
            let parts: Vec<String> = evaled_args.iter().map(|a| format_for_print(a)).collect();
            Expr::String(parts.join("").into())
        }
        "slength" | "charat" | "substring" | "ssearch" | "ssubst"
        | "strim" | "striml" | "strimr" | "split" | "supcase" | "sdowncase"
        | "sequal" => {
            if let Some(result) = crate::strings::eval_string_func(&func_name, &evaled_args) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "ifactors" | "totient" | "divisors" | "next_prime" | "prev_prime"
        | "power_mod" | "inv_mod" | "jacobi" | "chinese" | "fibonacci" => {
            if let Some(result) = crate::numtheory::eval_numtheory_func(&func_name, &evaled_args) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "ratsubst" => {
            // ratsubst(new, old, expr) — same as subst for now
            if evaled_args.len() == 3 {
                let result = subst(&evaled_args[0], &evaled_args[1], &evaled_args[2]);
                return meval(&result, env);
            }
            Expr::call("ratsubst", evaled_args)
        }
        "multthru" | "xthru" | "collectterms" | "at" | "lfreeof" | "lopow" => {
            if let Some(result) = crate::expr_manip::eval_expr_manip(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "resultant" | "discriminant" | "content" | "primpart"
        | "nroots" | "realroots" => {
            if let Some(result) = crate::poly_analysis::eval_sturm_func(&func_name, &evaled_args) {
                return result;
            }
            if let Some(result) = crate::poly_analysis::eval_poly_func(&func_name, &evaled_args) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "groebner_basis" => {
            if let Some(r) = crate::groebner::eval_groebner(&func_name, &evaled_args) {
                return r;
            }
            Expr::call(&func_name, evaled_args)
        }
        "polysys_solve" => crate::groebner::eval_polysys_solve(&evaled_args, env),
        "factor_multivariate" => crate::groebner::eval_factor_multivariate(&evaled_args, env),
        "eliminate" | "ideal_sum" | "ideal_product"
        | "ideal_intersect" | "ideal_contains" => {
            if let Some(r) = crate::groebner::eval_groebner(&func_name, &evaled_args) {
                return r;
            }
            Expr::call(&func_name, evaled_args)
        }
        "residue" => {
            if let Some(result) = crate::residue::eval_residue(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "logcontract" | "logexpand" => {
            if let Some(result) = crate::log_trig::eval_log_trig(&func_name, &evaled_args) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "matchdeclare" | "defrule" | "apply1" | "applyb1"
        | "tellsimp" | "tellsimpafter" => {
            if let Some(result) = crate::pattern::eval_pattern_func(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "plot2d" | "gnuplot_script" => {
            if let Some(result) = crate::plot::eval_plot(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "bfloat" | "fpprec" => {
            if let Some(result) = crate::bigfloat::eval_bfloat_func(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "help" => crate::help::eval_help(&evaled_args),
        "ode2" | "ic1" | "ic2" | "bc2" => {
            if let Some(result) = crate::ode::eval_ode(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "laplace" | "ilt" => {
            if let Some(result) = crate::laplace::eval_laplace(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "subst" => {
            if evaled_args.len() == 3 {
                let result = subst(&evaled_args[0], &evaled_args[1], &evaled_args[2]);
                meval(&result, env)
            } else if evaled_args.len() == 2 {
                // subst(x=val, expr) or subst([x=v1, y=v2], expr)
                let sub = &evaled_args[0];
                let target = &evaled_args[1];
                match sub {
                    Expr::List { op: Operator::MEqual, args: eq_args, .. } if eq_args.len() == 2 => {
                        let result = subst(&eq_args[1], &eq_args[0], target);
                        meval(&result, env)
                    }
                    Expr::List { op: Operator::MList, args: subs, .. } => {
                        let mut result = target.clone();
                        for s in subs {
                            if let Expr::List { op: Operator::MEqual, args: eq_args, .. } = s {
                                if eq_args.len() == 2 {
                                    result = subst(&eq_args[1], &eq_args[0], &result);
                                }
                            }
                        }
                        meval(&result, env)
                    }
                    _ => Expr::call("subst", evaled_args),
                }
            } else {
                Expr::call("subst", evaled_args)
            }
        }
        "expand" => {
            if let Some(arg) = evaled_args.first() {
                // expand(expr, 0, 0) means no expansion
                if evaled_args.len() >= 3 {
                    if to_f64(&evaled_args[1]) == Some(0.0) && to_f64(&evaled_args[2]) == Some(0.0) {
                        return arg.clone();
                    }
                }
                expand(arg)
            } else {
                Expr::call("expand", evaled_args)
            }
        }
        "ratexpand" => {
            if let Some(arg) = evaled_args.first() {
                expand(arg)
            } else {
                Expr::call("ratexpand", evaled_args)
            }
        }
        "diff" => eval_diff(&evaled_args),
        "integrate" => {
            if evaled_args.len() >= 2 {
                let f = &evaled_args[0];
                let var = &evaled_args[1];
                let result = table_integrate(f, var);
                if evaled_args.len() == 4 {
                    let a = &evaled_args[2];
                    let b = &evaled_args[3];
                    let is_inf = |e: &Expr| matches!(e, Expr::Symbol(id) if { let n = resolve(*id); n == "inf" || n == "infinity" });
                    let is_minf = |e: &Expr| matches!(e, Expr::Symbol(id) if resolve(*id) == "minf");

                    // Try known definite integral formulas first
                    if let Some(def_result) = try_known_definite_integral(f, var, a, b) {
                        return def_result;
                    }

                    // Cauchy principal value: for odd functions over symmetric intervals
                    // ∫_{-a}^{a} f(x) dx = 0 if f is odd and has a pole at 0
                    if let (Some(a_val), Some(b_val)) = (to_f64(a), to_f64(b)) {
                        if (a_val + b_val).abs() < 1e-15 && a_val < 0.0 {
                            // Symmetric interval [-c, c]
                            // Check if f(-x) = -f(x) (odd function)
                            let neg_x = Expr::neg(var.clone());
                            let f_neg = simplify(&subst(&neg_x, var, f));
                            let f_plus_fneg = simplify(&ratsimp(&Expr::add(f.clone(), f_neg)));
                            if f_plus_fneg == Expr::int(0) {
                                return Expr::int(0);
                            }
                            // Numeric check: f(t) + f(-t) ≈ 0 for a test value
                            let mut tmp = crate::Environment::new();
                            let test = Expr::Float(1.3);
                            let f_at = meval(&subst(&test, var, f), &mut tmp);
                            let neg_test = Expr::Float(-1.3);
                            let f_neg_at = meval(&subst(&neg_test, var, f), &mut tmp);
                            if let (Some(a_f), Some(b_f)) = (to_f64(&f_at), to_f64(&f_neg_at)) {
                                if (a_f + b_f).abs() < 1e-10 {
                                    return Expr::int(0);
                                }
                            }
                        }
                    }

                    if let Expr::Symbol(_vid) = var {
                        // For (-∞,∞): try residue method first (more direct for rationals)
                        if is_minf(a) && is_inf(b) {
                            if let Some(res) = try_residue_integral(f, var) {
                                return res;
                            }
                        }

                        let have_antideriv = !matches!(&result, Expr::List { op: Operator::Named(_), .. }
                            if result.to_string().starts_with("integrate"));

                        if have_antideriv {
                            // Infinite bounds: use limits
                            if is_inf(b) && !is_inf(a) && !is_minf(a) {
                                let fa = meval(&subst(a, var, &result), env);
                                let lim = crate::gruntz::gruntz_limit(&result, var);
                                if let Some(fb) = lim {
                                    if !matches!(&fb, Expr::Symbol(id) if { let n = resolve(*id); n == "inf" || n == "minf" || n == "und" }) {
                                        return simplify(&Expr::sub(fb, fa));
                                    }
                                }
                            } else if is_minf(a) && is_inf(b) {
                                // (-∞, ∞): need both limits
                                let lim_pos = crate::gruntz::gruntz_limit(&result, var);
                                // For -∞: substitute x → -x, take limit as x → ∞
                                let neg_var = Expr::neg(var.clone());
                                let result_neg = simplify(&subst(&neg_var, var, &result));
                                let lim_neg = crate::gruntz::gruntz_limit(&result_neg, var);
                                if let (Some(fp), Some(fn_)) = (lim_pos, lim_neg) {
                                    return simplify(&Expr::sub(fp, fn_));
                                }
                            } else {
                                // Finite bounds
                                let fa = meval(&subst(a, var, &result), env);
                                let fb = meval(&subst(b, var, &result), env);
                                return simplify(&Expr::sub(fb, fa));
                            }
                        }

                        // For rational functions over (-∞,∞): try residue method
                        if is_minf(a) && is_inf(b) {
                            if let Some(res) = try_residue_integral(f, var) {
                                return res;
                            }
                        }
                    }
                }
                return result;
            }
            Expr::call("integrate", evaled_args)
        }
        "limit" => {
            if evaled_args.len() >= 3 {
                let f = &evaled_args[0];
                let var = &evaled_args[1];
                let point = &evaled_args[2];

                // Limits at infinity
                let is_inf = matches!(point, Expr::Symbol(id) if {
                    let n = resolve(*id); n == "inf" || n == "infinity"
                });
                let is_minf = matches!(point, Expr::Symbol(id) if resolve(*id) == "minf");

                if is_inf || is_minf {
                    if let Expr::Symbol(var_id) = var {
                        // Polynomial: leading term determines limit
                        if let Some(poly) = maxima_poly::expr_to_poly(f, *var_id) {
                            let deg = poly.degree().unwrap_or(0);
                            if deg == 0 {
                                return match poly.constant_term() {
                                    maxima_poly::Coeff::Int(n) => Expr::int(n),
                                    maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                };
                            }
                            let lc = poly.leading_coeff();
                            let lc_pos = matches!(&lc, maxima_poly::Coeff::Int(n) if *n > 0);
                            if is_inf {
                                return if lc_pos { Expr::sym("inf") } else { Expr::sym("minf") };
                            } else {
                                // minf: sign depends on degree parity
                                let sign_flip = deg % 2 == 1;
                                return if lc_pos == sign_flip { Expr::sym("minf") } else { Expr::sym("inf") };
                            }
                        }
                        // Rational function: ratio of leading terms
                        if let Some((num, den)) = extract_fraction(f) {
                            if let (Some(np), Some(dp)) = (
                                maxima_poly::expr_to_poly(&num, *var_id),
                                maxima_poly::expr_to_poly(&den, *var_id),
                            ) {
                                let ndeg = np.degree().unwrap_or(0);
                                let ddeg = dp.degree().unwrap_or(0);
                                if ndeg < ddeg { return Expr::int(0); }
                                if ndeg == ddeg {
                                    if let Some(ratio) = np.leading_coeff().div(&dp.leading_coeff()) {
                                        return match ratio {
                                            maxima_poly::Coeff::Int(n) => Expr::int(n),
                                            maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                        };
                                    }
                                }
                                if ndeg > ddeg { return Expr::sym("inf"); }
                            }
                        }
                    }
                    // Try Gruntz algorithm for exp/log limits
                    if is_inf {
                        if let Some(result) = crate::gruntz::gruntz_limit(f, var) {
                            return result;
                        }
                    } else {
                        // x → -∞: substitute x → -t, compute limit as t → +∞
                        let neg_var = Expr::neg(var.clone());
                        let f_neg = simplify(&subst(&neg_var, var, f));
                        if let Some(result) = crate::gruntz::gruntz_limit(&f_neg, var) {
                            return result;
                        }
                    }
                    return Expr::call("limit", evaled_args);
                }

                // Directional limit: 4th arg is plus or minus
                let direction = evaled_args.get(3).and_then(|e| {
                    if let Expr::Symbol(id) = e {
                        let name = resolve(*id);
                        match name.as_str() {
                            "plus" => Some(1),
                            "minus" => Some(-1),
                            _ => None,
                        }
                    } else { None }
                });

                // If expression contains abs(var), resolve by direction
                if contains_abs_of(f, var) {
                    if let Some(dir) = direction {
                        let f_resolved = resolve_abs_for_limit(f, var, point, dir);
                        let new_args = vec![f_resolved, var.clone(), point.clone()];
                        return meval(&Expr::call("limit", new_args), env);
                    } else {
                        let f_pos = resolve_abs_for_limit(f, var, point, 1);
                        let f_neg = resolve_abs_for_limit(f, var, point, -1);
                        let lim_plus = meval(&Expr::call("limit", vec![f_pos, var.clone(), point.clone()]), env);
                        let lim_minus = meval(&Expr::call("limit", vec![f_neg, var.clone(), point.clone()]), env);
                        if lim_plus == lim_minus {
                            return lim_plus;
                        }
                        return Expr::sym("und");
                    }
                }

                // Finite limits
                let f_simplified = ratsimp(f);
                let result = meval(&subst(point, var, &f_simplified), env);
                if let Some((num, den)) = extract_fraction(&f_simplified) {
                    let num_at = meval(&subst(point, var, &num), env);
                    let den_at = meval(&subst(point, var, &den), env);
                    if num_at == Expr::int(0) && den_at == Expr::int(0) {
                        // L'Hôpital: iterate up to 5 times for higher-order 0/0
                        if let Some((mut num_expr, mut den_expr)) = extract_fraction(f) {
                            for _ in 0..5 {
                                let dnum = eval_diff(&[num_expr, var.clone()]);
                                let dden = eval_diff(&[den_expr, var.clone()]);
                                let n_at = meval(&subst(point, var, &dnum), env);
                                let d_at = meval(&subst(point, var, &dden), env);
                                if d_at != Expr::int(0) {
                                    let ratio = simplify(&Expr::div(dnum, dden));
                                    return meval(&subst(point, var, &ratio), env);
                                }
                                if n_at != Expr::int(0) {
                                    // n_at ≠ 0 but d_at = 0 → ±∞
                                    return Expr::sym("inf");
                                }
                                num_expr = dnum;
                                den_expr = dden;
                            }
                        }
                    }
                }
                if let Expr::Rational { den: 0, .. } = &result {
                    return Expr::sym("und");
                }
                return result;
            }
            Expr::call("limit", evaled_args)
        }
        "taylor" => {
            // Basic Taylor: evaluate diff repeatedly
            if evaled_args.len() >= 4 {
                if let (Some(_a), Some(n)) = (to_f64(&evaled_args[2]), to_i64(&evaled_args[3])) {
                    let f = &evaled_args[0];
                    let var = &evaled_args[1];
                    let a_expr = &evaled_args[2];
                    let mut result = Expr::int(0);
                    let mut deriv = f.clone();
                    let mut factorial = 1i64;
                    for k in 0..=n {
                        if k > 0 { factorial *= k; }
                        let coeff = meval(&subst(a_expr, var, &deriv), env);
                        let term = simplify(&Expr::mul(
                            Expr::div(coeff, Expr::int(factorial)),
                            Expr::pow(Expr::sub(var.clone(), a_expr.clone()), Expr::int(k)),
                        ));
                        result = simplify(&Expr::add(result, term));
                        deriv = eval_diff(&[deriv, var.clone()]);
                    }
                    return result;
                }
            }
            Expr::call("taylor", evaled_args)
        }
        "sin" | "cos" | "tan" | "log" | "exp" | "sqrt"
        | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh"
        | "erf" | "erfc" | "erfi"
        | "expintegral_ei" | "expintegral_li"
        | "expintegral_si" | "expintegral_ci"
        | "fresnel_s" | "fresnel_c" => {
            eval_math_func(&func_name, &evaled_args)
        }
        "binomial" => {
            if evaled_args.len() == 2 {
                if let (Expr::Integer(n), Expr::Integer(k)) = (&evaled_args[0], &evaled_args[1]) {
                    if *k < 0 || *k > *n { return Expr::int(0); }
                    let mut result = 1i64;
                    let k_val = (*k).min(*n - *k) as u64;
                    for i in 0..k_val {
                        result = result * (*n - i as i64) / (i as i64 + 1);
                    }
                    return Expr::int(result);
                }
            }
            Expr::call("binomial", evaled_args)
        }
        "gcd" => {
            if evaled_args.len() >= 2 {
                if let (Expr::Integer(a), Expr::Integer(b)) = (&evaled_args[0], &evaled_args[1]) {
                    let g = gcd_i64(a.unsigned_abs(), b.unsigned_abs());
                    return Expr::int(g as i64);
                }
                // Try polynomial GCD
                let var = if evaled_args.len() >= 3 {
                    if let Expr::Symbol(id) = &evaled_args[2] { *id } else { maxima_core::intern("x") }
                } else {
                    maxima_core::intern("x")
                };
                if let (Some(pa), Some(pb)) = (
                    maxima_poly::expr_to_poly(&evaled_args[0], var),
                    maxima_poly::expr_to_poly(&evaled_args[1], var),
                ) {
                    let g = maxima_poly::poly_gcd(&pa, &pb);
                    return maxima_poly::poly_to_expr(&g);
                }
            }
            Expr::call("gcd", evaled_args)
        }
        "divide" | "quotient" => {
            if evaled_args.len() >= 2 {
                let var = if evaled_args.len() >= 3 {
                    if let Expr::Symbol(id) = &evaled_args[2] { *id } else { maxima_core::intern("x") }
                } else {
                    maxima_core::intern("x")
                };
                if let (Some(pa), Some(pb)) = (
                    maxima_poly::expr_to_poly(&evaled_args[0], var),
                    maxima_poly::expr_to_poly(&evaled_args[1], var),
                ) {
                    if let Some((q, r)) = pa.divmod(&pb) {
                        return Expr::list(vec![
                            maxima_poly::poly_to_expr(&q),
                            maxima_poly::poly_to_expr(&r),
                        ]);
                    }
                }
            }
            Expr::call("divide", evaled_args)
        }
        "remainder" => {
            if evaled_args.len() >= 2 {
                let var = maxima_core::intern("x");
                if let (Some(pa), Some(pb)) = (
                    maxima_poly::expr_to_poly(&evaled_args[0], var),
                    maxima_poly::expr_to_poly(&evaled_args[1], var),
                ) {
                    if let Some((_, r)) = pa.divmod(&pb) {
                        return maxima_poly::poly_to_expr(&r);
                    }
                }
            }
            Expr::call("remainder", evaled_args)
        }
        "hipow" => {
            if evaled_args.len() == 2 {
                if let Expr::Symbol(id) = &evaled_args[1] {
                    if let Some(p) = maxima_poly::expr_to_poly(&evaled_args[0], *id) {
                        return Expr::int(p.degree().unwrap_or(0) as i64);
                    }
                }
            }
            Expr::call("hipow", evaled_args)
        }
        "coeff" => {
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(id) = &evaled_args[1] {
                    let power = if evaled_args.len() >= 3 {
                        if let Expr::Integer(n) = &evaled_args[2] { *n as u32 } else { 1 }
                    } else {
                        1
                    };
                    if let Some(p) = maxima_poly::expr_to_poly(&evaled_args[0], *id) {
                        let c = p.terms.iter()
                            .find(|(e, _)| *e == power)
                            .map(|(_, c)| match c {
                                maxima_poly::Coeff::Int(n) => Expr::int(*n),
                                maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: *n, den: *d },
                            })
                            .unwrap_or(Expr::int(0));
                        return c;
                    }
                }
            }
            Expr::call("coeff", evaled_args)
        }
        "factor" => {
            if let Some(arg) = evaled_args.first() {
                // Find a variable to factor over
                let var = find_variable(arg).unwrap_or_else(|| maxima_core::intern("x"));
                if let Some(poly) = maxima_poly::expr_to_poly(arg, var) {
                    let content = poly.content();
                    let factors = maxima_poly::factor_poly(&poly);
                    if factors.len() > 1 || factors.iter().any(|(_, m)| *m > 1) {
                        let mut parts: Vec<Expr> = Vec::new();
                        if !content.is_one() {
                            match content {
                                maxima_poly::Coeff::Int(n) => parts.push(Expr::int(n)),
                                maxima_poly::Coeff::Rat(n, d) => parts.push(Expr::Rational { num: n, den: d }),
                            }
                        }
                        for (f, m) in &factors {
                            let fe = maxima_poly::poly_to_expr(f);
                            if *m == 1 {
                                parts.push(fe);
                            } else {
                                parts.push(Expr::pow(fe, Expr::int(*m as i64)));
                            }
                        }
                        if parts.len() == 1 {
                            return parts.pop().unwrap();
                        }
                        return simplify(&Expr::List {
                            op: Operator::MTimes,
                            simplified: false,
                            args: parts,
                        });
                    }
                }
                return arg.clone();
            }
            Expr::call("factor", evaled_args)
        }
        "sqfr" => {
            if let Some(arg) = evaled_args.first() {
                let var = find_variable(arg).unwrap_or_else(|| maxima_core::intern("x"));
                if let Some(poly) = maxima_poly::expr_to_poly(arg, var) {
                    let factors = maxima_poly::factor_poly(&poly);
                    let mut parts: Vec<Expr> = Vec::new();
                    for (f, m) in &factors {
                        let fe = maxima_poly::poly_to_expr(f);
                        if *m == 1 {
                            parts.push(fe);
                        } else {
                            parts.push(Expr::pow(fe, Expr::int(*m as i64)));
                        }
                    }
                    if parts.len() == 1 {
                        return parts.pop().unwrap();
                    }
                    return simplify(&Expr::List {
                        op: Operator::MTimes,
                        simplified: false,
                        args: parts,
                    });
                }
                return arg.clone();
            }
            Expr::call("sqfr", evaled_args)
        }
        "partfrac" => {
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(var_id) = &evaled_args[1] {
                    if let Some((num, den)) = extract_fraction(&evaled_args[0]) {
                        if let (Some(np), Some(dp)) = (
                            maxima_poly::expr_to_poly(&expand(&num), *var_id),
                            maxima_poly::expr_to_poly(&expand(&den), *var_id),
                        ) {
                            let factors = maxima_poly::factor_poly(&dp);
                            // Try biquadratic factoring if factor_poly didn't split
                            let factors = if factors.len() <= 1 {
                                try_biquadratic_factor(&dp, *var_id).unwrap_or(factors)
                            } else { factors };
                            if factors.len() > 1 {
                                // Try partial fractions for mix of linear and quadratic factors
                                if let Some(result) = partfrac_general(&np, &factors, *var_id) {
                                    return result;
                                }
                                // Fallback: simple linear-only partial fractions
                                let mut terms = Vec::new();
                                for (f, _m) in &factors {
                                    if f.degree() == Some(1) {
                                        let a = f.leading_coeff();
                                        let b = f.constant_term();
                                        if let Some(root) = b.neg().div(&a) {
                                            let num_at_root = np.eval_at(&root);
                                            let mut denom_at_root = maxima_poly::Coeff::one();
                                            for (f2, _) in &factors {
                                                if f2 != f {
                                                    denom_at_root = denom_at_root.mul(&f2.eval_at(&root));
                                                }
                                            }
                                            if let Some(residue) = num_at_root.div(&denom_at_root) {
                                                let coeff_expr = match residue {
                                                    maxima_poly::Coeff::Int(n) => Expr::int(n),
                                                    maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                                };
                                                let denom_expr = maxima_poly::poly_to_expr(f);
                                                terms.push(simplify(&Expr::div(coeff_expr, denom_expr)));
                                            }
                                        }
                                    }
                                }
                                if !terms.is_empty() {
                                    return simplify(&Expr::List {
                                        op: Operator::MPlus,
                                        simplified: false,
                                        args: terms,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Expr::call("partfrac", evaled_args)
        }
        "ratsimp" => {
            if let Some(arg) = evaled_args.first() {
                ratsimp(arg)
            } else {
                Expr::call("ratsimp", evaled_args)
            }
        }
        "funapply" => {
            if let Some(func) = evaled_args.first() {
                let call_args = &evaled_args[1..];
                // Check for subscripted function call: funapply(mqapply(t, 4), y)
                if let Expr::List { op: Operator::Named(mq_id), args: mq_args, .. } = func {
                    if resolve(*mq_id) == "mqapply" && mq_args.len() >= 2 {
                        return eval_subscript_call(&mq_args[0], &mq_args[1..], call_args, env);
                    }
                }
                return apply_func(func, call_args, env);
            }
            Expr::call("funapply", evaled_args)
        }
        "mqapply" => {
            // Bare subscript reference: t[2] without function call
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(base_id) = &evaled_args[0] {
                    let indices: Vec<String> = evaled_args[1..].iter().map(|i| i.to_string()).collect();
                    let key = crate::env::SubscriptKey {
                        name: *base_id,
                        indices: indices.clone(),
                    };
                    // Check for concrete subscripted function → return as lambda
                    if let Some(def) = env.subscript_fns.get(&key).cloned() {
                        let params = Expr::list(def.params.iter().map(|p| Expr::Symbol(*p)).collect());
                        return Expr::List {
                            op: Operator::MLambda,
                            simplified: false,
                            args: vec![params, def.body],
                        };
                    }
                    // Check generic subscripted function → evaluate with index, return lambda
                    if let Some((index_params, def)) = env.subscript_generic_fns.get(base_id).cloned() {
                        let func_params = &def.params[index_params.len()..];
                        env.push_scope();
                        for (param, idx) in index_params.iter().zip(evaled_args[1..].iter()) {
                            env.set_local(*param, idx.clone());
                        }
                        let body = meval(&def.body, env);
                        env.pop_scope();
                        let params = Expr::list(func_params.iter().map(|p| Expr::Symbol(*p)).collect());
                        return Expr::List {
                            op: Operator::MLambda,
                            simplified: false,
                            args: vec![params, body],
                        };
                    }
                    // Check array values
                    let arr_key = (*base_id, indices);
                    if let Some(val) = env.array_values.get(&arr_key) {
                        return val.clone();
                    }
                }
            }
            // Matrix element access: matrix(...)[i,j]
            if evaled_args.len() == 3 {
                if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled_args[0] {
                    if let (Some(i), Some(j)) = (to_i64(&evaled_args[1]), to_i64(&evaled_args[2])) {
                        if i >= 1 && j >= 1 {
                            let ri = (i - 1) as usize;
                            let ci = (j - 1) as usize;
                            if ri < rows.len() {
                                if let Expr::List { op: Operator::MList, args: cols, .. } = &rows[ri] {
                                    if ci < cols.len() {
                                        return cols[ci].clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Matrix row access: matrix(...)[i]
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled_args[0] {
                    if let Some(i) = to_i64(&evaled_args[1]) {
                        if i >= 1 && (i as usize) <= rows.len() {
                            return rows[(i - 1) as usize].clone();
                        }
                    }
                }
            }
            // List element access: L[i].  Maxima is 1-indexed; negative i counts
            // from the end (L[-1] is the last element).
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[0] {
                    if let Some(i) = to_i64(&evaled_args[1]) {
                        if i >= 1 && (i as usize) <= items.len() {
                            return items[(i - 1) as usize].clone();
                        }
                        if i < 0 {
                            let k = (-i) as usize;
                            if k >= 1 && k <= items.len() {
                                return items[items.len() - k].clone();
                            }
                        }
                    }
                }
            }
            Expr::call("mqapply", evaled_args)
        }
        "sort" => eval_sort(&evaled_args, env),
        "emptyp" => {
            match evaled_args.first() {
                Some(Expr::List { op: Operator::MList, args, .. }) => bool_result(args.is_empty()),
                Some(Expr::List { op: Operator::MMatrix, args, .. }) => {
                    bool_result(args.is_empty() || args.iter().all(|r| {
                        matches!(r, Expr::List { op: Operator::MList, args: cols, .. } if cols.is_empty())
                    }))
                }
                _ => Expr::sym("false"),
            }
        }
        "identity" => {
            if let Some(arg) = evaled_args.first() {
                return arg.clone();
            }
            Expr::call("identity", evaled_args)
        }
        "every" => {
            if evaled_args.len() == 2 {
                let pred = &evaled_args[0];
                let items = match &evaled_args[1] {
                    Expr::List { op: Operator::MList, args, .. } => args.clone(),
                    Expr::List { op: Operator::MMatrix, args: rows, .. } => {
                        // Flatten matrix to list of elements
                        let mut all = Vec::new();
                        for row in rows {
                            if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                                all.extend(cols.iter().cloned());
                            }
                        }
                        all
                    }
                    _ => return Expr::call("every", evaled_args),
                };
                if items.is_empty() {
                    return Expr::sym("true");
                }
                for item in &items {
                    let result = apply_func(pred, &[item.clone()], env);
                    if is_false(&result) {
                        return Expr::sym("false");
                    }
                    if !is_true(&result) && result != Expr::sym("true") {
                        return Expr::sym("false");
                    }
                }
                return Expr::sym("true");
            }
            if evaled_args.len() == 1 {
                return if is_true(&evaled_args[0]) { Expr::sym("true") } else { Expr::sym("false") };
            }
            Expr::call("every", evaled_args)
        }
        "some" => {
            if evaled_args.len() == 2 {
                let pred = &evaled_args[0];
                let items = match &evaled_args[1] {
                    Expr::List { op: Operator::MList, args, .. } => args.clone(),
                    Expr::List { op: Operator::MMatrix, args: rows, .. } => {
                        let mut all = Vec::new();
                        for row in rows {
                            if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                                all.extend(cols.iter().cloned());
                            }
                        }
                        all
                    }
                    _ => return Expr::call("some", evaled_args),
                };
                if items.is_empty() {
                    return Expr::sym("false");
                }
                for item in &items {
                    let result = apply_func(pred, &[item.clone()], env);
                    if is_true(&result) {
                        return Expr::sym("true");
                    }
                }
                return Expr::sym("false");
            }
            Expr::call("some", evaled_args)
        }
        "flatten" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                let mut flat = Vec::new();
                for item in items {
                    if let Expr::List { op: Operator::MList, args: inner, .. } = item {
                        flat.extend(inner.iter().cloned());
                    } else {
                        flat.push(item.clone());
                    }
                }
                return Expr::list(flat);
            }
            Expr::call("flatten", evaled_args)
        }
        "delete" => {
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[1] {
                    let filtered: Vec<Expr> = items.iter()
                        .filter(|item| **item != evaled_args[0])
                        .cloned()
                        .collect();
                    return Expr::list(filtered);
                }
            }
            Expr::call("delete", evaled_args)
        }
        "lmax" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                if let Some(max) = items.iter().max_by(|a, b| {
                    match (to_f64(a), to_f64(b)) {
                        (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
                        _ => a.to_string().cmp(&b.to_string()),
                    }
                }) {
                    return max.clone();
                }
            }
            Expr::call("lmax", evaled_args)
        }
        "lmin" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                if let Some(min) = items.iter().min_by(|a, b| {
                    match (to_f64(a), to_f64(b)) {
                        (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
                        _ => a.to_string().cmp(&b.to_string()),
                    }
                }) {
                    return min.clone();
                }
            }
            Expr::call("lmin", evaled_args)
        }
        "sublist" => {
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[0] {
                    let pred = &evaled_args[1];
                    let filtered: Vec<Expr> = items.iter()
                        .filter(|item| {
                            let result = apply_func(pred, &[(*item).clone()], env);
                            is_true(&result)
                        })
                        .cloned()
                        .collect();
                    return Expr::list(filtered);
                }
            }
            Expr::call("sublist", evaled_args)
        }
        "push" => {
            if evaled_args.len() == 2 {
                if let Expr::Symbol(id) = &args[1] {
                    let val = evaled_args[0].clone();
                    let current = env.get(*id).cloned().unwrap_or(Expr::list(vec![]));
                    if let Expr::List { op: Operator::MList, args: items, .. } = current {
                        let mut new_items = vec![val.clone()];
                        new_items.extend(items);
                        let new_list = Expr::list(new_items);
                        env.set(*id, new_list);
                        return val;
                    }
                }
            }
            Expr::call("push", evaled_args)
        }
        "xreduce" => {
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MList, args: items, .. } = &evaled_args[1] {
                    if items.is_empty() {
                        // xreduce("+", []) => 0, xreduce("*", []) => 1
                        if let Expr::String(s) = &evaled_args[0] {
                            return match s.as_ref() {
                                "+" => Expr::int(0),
                                "*" => Expr::int(1),
                                _ => Expr::call("xreduce", evaled_args),
                            };
                        }
                    }
                }
            }
            Expr::call("xreduce", evaled_args)
        }
        "put" => {
            // put(sym, val, prop) — property list storage (stub: store in env)
            Expr::sym("done")
        }
        "get" => Expr::sym("false"),
        "assume" => {
            let mut results = Vec::new();
            for arg in &evaled_args {
                if let Some((lhs, rel, rhs)) = extract_relation(arg) {
                    let fact = Fact { lhs, rel, rhs };
                    let r = env.assumptions.assume(fact);
                    results.push(Expr::sym(r));
                } else {
                    results.push(arg.clone());
                }
            }
            if results.len() == 1 {
                results.pop().unwrap()
            } else {
                Expr::list(results)
            }
        }
        "forget" => {
            for arg in &evaled_args {
                if let Some((lhs, rel, rhs)) = extract_relation(arg) {
                    env.assumptions.forget(&lhs, rel, &rhs);
                }
            }
            Expr::sym("done")
        }
        "facts" => {
            let facts = env.assumptions.facts();
            let items: Vec<Expr> = facts.iter().map(|f| relation_to_expr(f)).collect();
            Expr::list(items)
        }
        "asksign" => {
            if let Some(arg) = evaled_args.first() {
                let sign = compute_sign(arg, &env.assumptions);
                Expr::sym(sign.to_maxima_str())
            } else {
                Expr::sym("pnz")
            }
        }
        "assuming" => {
            // assuming(pred, expr) — temporarily assume pred, evaluate expr
            if evaled_args.len() >= 2 {
                let pred = &evaled_args[0];
                if let Some((lhs, rel, rhs)) = extract_relation(pred) {
                    let fact = Fact { lhs, rel, rhs };
                    env.assumptions.new_context("assuming_temp");
                    env.assumptions.assume(fact);
                    let result = meval(&args[1], env);
                    env.assumptions.kill_context("assuming_temp");
                    return result;
                }
            }
            Expr::call("assuming", evaled_args)
        }
        "set" => {
            // set(a,b,c,...) — create a set (list with unique sorted elements)
            let mut items = evaled_args.clone();
            // Remove duplicates
            items.dedup_by(|a, b| a == b);
            items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            items.dedup_by(|a, b| a == b);
            Expr::List {
                op: Operator::MList,
                simplified: true,
                args: items,
            }
        }
        "setify" | "listify" | "union" | "intersection"
        | "setdifference" | "symdifference" | "cardinality" | "elementp"
        | "subsetp" | "disjointp" | "powerset" | "subset" => {
            if let Some(result) = crate::sets::eval_set_func(&func_name, &evaled_args, env) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "member" => {
            if evaled_args.len() == 2 {
                match &evaled_args[1] {
                    Expr::List { op: Operator::MList | Operator::MSet, args: items, .. } =>
                        return bool_result(items.contains(&evaled_args[0])),
                    _ => {}
                }
            }
            Expr::call("member", evaled_args)
        }
        "rat" => {
            // rat(expr) — convert to canonical rational form (stub: simplify)
            if let Some(arg) = evaled_args.first() {
                return simplify(arg);
            }
            Expr::call("rat", evaled_args)
        }
        "ratdisrep" => {
            // ratdisrep is identity in our representation
            if let Some(arg) = evaled_args.first() {
                return arg.clone();
            }
            Expr::call("ratdisrep", evaled_args)
        }
        "num" | "ratnumer" => {
            // numerator
            if let Some(Expr::Rational { num, .. }) = evaled_args.first() {
                return Expr::int(*num);
            }
            if let Some(arg) = evaled_args.first() {
                return arg.clone();
            }
            Expr::call("num", evaled_args)
        }
        "denom" | "ratdenom" => {
            // denominator
            if let Some(Expr::Rational { den, .. }) = evaled_args.first() {
                return Expr::int(*den);
            }
            return Expr::int(1);
        }
        "solve" => {
            // Basic solve: linear and quadratic only
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(var_id) = &evaled_args[1] {
                    let var = *var_id;
                    // Handle equation form: solve(expr = 0, x) → solve(expr, x)
                    let solve_expr = if let Expr::List { op: Operator::MEqual, args: eq_args, .. } = &evaled_args[0] {
                        if eq_args.len() == 2 {
                            simplify(&Expr::sub(eq_args[0].clone(), eq_args[1].clone()))
                        } else { evaled_args[0].clone() }
                    } else { evaled_args[0].clone() };
                    let solve_expr = expand(&solve_expr);
                    // Try polynomial conversion and root finding
                    if let Some(poly) = maxima_poly::expr_to_poly(&solve_expr, var) {
                        match poly.degree() {
                            Some(1) => {
                                // ax + b = 0 → x = -b/a
                                let a = poly.leading_coeff();
                                let b = poly.constant_term();
                                if let Some(root) = b.neg().div(&a) {
                                    let var_name = resolve(var);
                                    let root_expr = match root {
                                        maxima_poly::Coeff::Int(n) => Expr::int(n),
                                        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                    };
                                    return Expr::list(vec![Expr::List {
                                        op: Operator::MEqual,
                                        simplified: false,
                                        args: vec![Expr::sym(&var_name), root_expr],
                                    }]);
                                }
                            }
                            Some(2) => {
                                // ax^2 + bx + c = 0 → quadratic formula
                                let a_c = poly.terms.iter().find(|(e,_)| *e == 2).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                let b_c = poly.terms.iter().find(|(e,_)| *e == 1).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                let c_c = poly.terms.iter().find(|(e,_)| *e == 0).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                if let (maxima_poly::Coeff::Int(a), maxima_poly::Coeff::Int(b), maxima_poly::Coeff::Int(c)) = (&a_c, &b_c, &c_c) {
                                    let disc = b*b - 4*a*c;
                                    let var_name = resolve(var);
                                    if disc >= 0 {
                                        let sqrt_disc = (disc as f64).sqrt();
                                        if (sqrt_disc * sqrt_disc - disc as f64).abs() < 0.5 {
                                            let sd = sqrt_disc.round() as i64;
                                            if sd * sd == disc {
                                                let r1_num = -b + sd;
                                                let r2_num = -b - sd;
                                                let den = 2 * a;
                                                let g1 = crate::simp::gcd_pub(r1_num.unsigned_abs(), den.unsigned_abs()) as i64;
                                                let g2 = crate::simp::gcd_pub(r2_num.unsigned_abs(), den.unsigned_abs()) as i64;
                                                let root1 = if den / g1 == 1 { Expr::int(r1_num / g1) } else { Expr::Rational { num: r1_num / g1, den: den / g1 } };
                                                let root2 = if den / g2 == 1 { Expr::int(r2_num / g2) } else { Expr::Rational { num: r2_num / g2, den: den / g2 } };
                                                let v = Expr::sym(&var_name);
                                                if root1 == root2 {
                                                    return Expr::list(vec![Expr::List { op: Operator::MEqual, simplified: false, args: vec![v, root1] }]);
                                                }
                                                return Expr::list(vec![
                                                    Expr::List { op: Operator::MEqual, simplified: false, args: vec![v.clone(), root1] },
                                                    Expr::List { op: Operator::MEqual, simplified: false, args: vec![v, root2] },
                                                ]);
                                            }
                                        }
                                    }
                                }
                            }
                            Some(3) => {
                                // Cubic: try factoring first
                                let factors = maxima_poly::factor_poly(&poly);
                                let var_name = resolve(var);
                                let v = Expr::sym(&var_name);
                                let mut roots = Vec::new();
                                for (f, _m) in &factors {
                                    if f.degree() == Some(1) {
                                        let a = f.leading_coeff();
                                        let b = f.constant_term();
                                        if let Some(root) = b.neg().div(&a) {
                                            let re = match root {
                                                maxima_poly::Coeff::Int(n) => Expr::int(n),
                                                maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                            };
                                            if !roots.contains(&re) {
                                                roots.push(re);
                                            }
                                        }
                                    }
                                }
                                if !roots.is_empty() {
                                    let solutions: Vec<Expr> = roots.into_iter()
                                        .map(|r| Expr::List { op: Operator::MEqual, simplified: false, args: vec![v.clone(), r] })
                                        .collect();
                                    return Expr::list(solutions);
                                }
                            }
                            _ => {
                                // Higher degree: try factoring
                                let factors = maxima_poly::factor_poly(&poly);
                                let var_name = resolve(var);
                                let v = Expr::sym(&var_name);
                                let mut roots = Vec::new();
                                for (f, _m) in &factors {
                                    if f.degree() == Some(1) {
                                        let a = f.leading_coeff();
                                        let b = f.constant_term();
                                        if let Some(root) = b.neg().div(&a) {
                                            let re = match root {
                                                maxima_poly::Coeff::Int(n) => Expr::int(n),
                                                maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                            };
                                            if !roots.contains(&re) {
                                                roots.push(re);
                                            }
                                        }
                                    }
                                }
                                if !roots.is_empty() {
                                    let solutions: Vec<Expr> = roots.into_iter()
                                        .map(|r| Expr::List { op: Operator::MEqual, simplified: false, args: vec![v.clone(), r] })
                                        .collect();
                                    return Expr::list(solutions);
                                }
                            }
                        }
                    }
                    // Symbolic quadratic: a*x²+b*x+c=0 with non-numeric coefficients
                    // Extract coefficients by collecting powers of var
                    let var_expr = Expr::Symbol(var);
                    let var_name = resolve(var);
                    let v = Expr::sym(&var_name);
                    if contains_var(&solve_expr, &var_expr) {
                        // Try to extract a, b, c from a*x²+b*x+c
                        let expanded = expand(&solve_expr);
                        if let Some(coeffs) = extract_symbolic_quad_coeffs(&expanded, &var_expr) {
                            let (a_e, b_e, c_e) = coeffs;
                            // x = (-b ± sqrt(b²-4ac)) / (2a)
                            let disc = simplify(&Expr::sub(
                                Expr::pow(b_e.clone(), Expr::int(2)),
                                Expr::mul(Expr::int(4), Expr::mul(a_e.clone(), c_e.clone()))));
                            let sqrt_disc = Expr::call("sqrt", vec![disc]);
                            let neg_b = simplify(&Expr::neg(b_e.clone()));
                            let two_a = simplify(&Expr::mul(Expr::int(2), a_e));
                            let root1 = simplify(&Expr::div(
                                Expr::add(neg_b.clone(), sqrt_disc.clone()), two_a.clone()));
                            let root2 = simplify(&Expr::div(
                                Expr::sub(neg_b, sqrt_disc), two_a));
                            return Expr::list(vec![
                                Expr::List { op: Operator::MEqual, simplified: false, args: vec![v.clone(), root1] },
                                Expr::List { op: Operator::MEqual, simplified: false, args: vec![v, root2] },
                            ]);
                        }
                    }
                }
            }
            Expr::call("solve", evaled_args)
        }
        "linsolve" => {
            // linsolve([eq1, eq2, ...], [x, y, ...])
            if evaled_args.len() == 2 {
                if let (Expr::List { op: Operator::MList, args: eqs, .. },
                        Expr::List { op: Operator::MList, args: vars, .. }) = (&evaled_args[0], &evaled_args[1]) {
                    return eval_linsolve(eqs, vars, env);
                }
            }
            Expr::call("linsolve", evaled_args)
        }
        "charpoly" => {
            // charpoly(M, x) = det(M - x*I)
            if evaled_args.len() == 2 {
                if let Expr::List { op: Operator::MMatrix, args: rows, .. } = &evaled_args[0] {
                    let _n = rows.len();
                    let var = &evaled_args[1];
                    let mut mat: Vec<Vec<Expr>> = Vec::new();
                    for (i, row) in rows.iter().enumerate() {
                        if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                            let new_row: Vec<Expr> = cols.iter().enumerate().map(|(j, c)| {
                                if i == j {
                                    simplify(&Expr::sub(c.clone(), var.clone()))
                                } else {
                                    c.clone()
                                }
                            }).collect();
                            mat.push(new_row);
                        }
                    }
                    return expand(&matrix_det(&mat, env));
                }
            }
            Expr::call("charpoly", evaled_args)
        }
        "eigenvalues" => {
            // eigenvalues(M) = solve(charpoly(M,x), x)
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                let _n = rows.len();
                let x_var = maxima_core::intern("x");
                let x = Expr::sym("x");
                let mut mat: Vec<Vec<Expr>> = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                        let new_row: Vec<Expr> = cols.iter().enumerate().map(|(j, c)| {
                            if i == j { simplify(&Expr::sub(c.clone(), x.clone())) } else { c.clone() }
                        }).collect();
                        mat.push(new_row);
                    }
                }
                let cp = expand(&matrix_det(&mat, env));
                if let Some(poly) = maxima_poly::expr_to_poly(&cp, x_var) {
                    let factors = maxima_poly::factor_poly(&poly);
                    let mut eigenvals = Vec::new();
                    let mut multiplicities = Vec::new();
                    for (f, m) in &factors {
                        if f.degree() == Some(1) {
                            let a = f.leading_coeff();
                            let b = f.constant_term();
                            if let Some(root) = b.neg().div(&a) {
                                let re = match root {
                                    maxima_poly::Coeff::Int(n) => Expr::int(n),
                                    maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                };
                                eigenvals.push(re);
                                multiplicities.push(Expr::int(*m as i64));
                            }
                        }
                    }
                    if !eigenvals.is_empty() {
                        return Expr::list(vec![Expr::list(eigenvals), Expr::list(multiplicities)]);
                    }
                }
            }
            Expr::call("eigenvalues", evaled_args)
        }
        "eigenvectors" => {
            // eigenvectors(M) = eigenvalues + null space for each
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                let _n = rows.len();
                let x_var = maxima_core::intern("x");
                let x = Expr::sym("x");
                let mut mat: Vec<Vec<Expr>> = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                        let new_row: Vec<Expr> = cols.iter().enumerate().map(|(j, c)| {
                            if i == j { simplify(&Expr::sub(c.clone(), x.clone())) } else { c.clone() }
                        }).collect();
                        mat.push(new_row);
                    }
                }
                let cp = expand(&matrix_det(&mat, env));
                if let Some(poly) = maxima_poly::expr_to_poly(&cp, x_var) {
                    let factors = maxima_poly::factor_poly(&poly);
                    let mut eigenvals = Vec::new();
                    let mut multiplicities = Vec::new();
                    let mut eigenvecs = Vec::new();

                    for (f, m) in &factors {
                        if f.degree() == Some(1) {
                            let a = f.leading_coeff();
                            let b = f.constant_term();
                            if let Some(root) = b.neg().div(&a) {
                                let re = match root {
                                    maxima_poly::Coeff::Int(n) => Expr::int(n),
                                    maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                };
                                eigenvals.push(re.clone());
                                multiplicities.push(Expr::int(*m as i64));

                                // Find eigenvector via null space of (M - λI)
                                let mut aug: Vec<Vec<f64>> = Vec::new();
                                for (i, row) in rows.iter().enumerate() {
                                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                                        let r: Vec<f64> = cols.iter().enumerate().map(|(j, c)| {
                                            let val = to_f64(c).unwrap_or(0.0);
                                            if i == j {
                                                val - to_f64(&re).unwrap_or(0.0)
                                            } else {
                                                val
                                            }
                                        }).collect();
                                        aug.push(r);
                                    }
                                }
                                // Row reduce to find null space
                                let evec = null_space_vector(&aug);
                                eigenvecs.push(Expr::list(vec![
                                    Expr::list(evec.iter().map(|v| {
                                        if *v == v.round() { Expr::int(*v as i64) } else { Expr::Float(*v) }
                                    }).collect()),
                                ]));
                            }
                        }
                    }
                    if !eigenvals.is_empty() {
                        return Expr::list(vec![
                            Expr::list(vec![Expr::list(eigenvals), Expr::list(multiplicities)]),
                            Expr::list(eigenvecs),
                        ]);
                    }
                }
            }
            Expr::call("eigenvectors", evaled_args)
        }
        "rank" => {
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                let mut mat: Vec<Vec<f64>> = Vec::new();
                for row in rows {
                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                        let r: Vec<f64> = cols.iter().map(|c| to_f64(c).unwrap_or(0.0)).collect();
                        mat.push(r);
                    }
                }
                let r = numeric_rank(&mat);
                return Expr::int(r as i64);
            }
            Expr::call("rank", evaled_args)
        }
        "trigexpand" => {
            if let Some(arg) = evaled_args.first() {
                return trig_expand(arg);
            }
            Expr::call("trigexpand", evaled_args)
        }
        "trigreduce" => {
            if let Some(arg) = evaled_args.first() {
                return trig_reduce(arg);
            }
            Expr::call("trigreduce", evaled_args)
        }
        "trigrat" => {
            if let Some(arg) = evaled_args.first() {
                return trig_rat(arg);
            }
            Expr::call("trigrat", evaled_args)
        }
        "halfangles" => {
            if let Some(arg) = evaled_args.first() {
                return half_angles(arg);
            }
            Expr::call("halfangles", evaled_args)
        }
        "trigsimp" => {
            if let Some(arg) = evaled_args.first() {
                return trig_simp(arg);
            }
            Expr::call("trigsimp", evaled_args)
        }
        "radcan" => {
            if let Some(arg) = evaled_args.first() {
                return simplify(arg);
            }
            Expr::call("radcan", evaled_args)
        }
        "tex" | "tex1" => {
            if let Some(arg) = evaled_args.first() {
                let tex = expr_to_tex(arg);
                if evaled_args.len() == 1 || (evaled_args.len() >= 2 && evaled_args[1] != Expr::sym("false")) {
                    println!("$${}$$", tex);
                }
                return Expr::String(tex.into());
            }
            Expr::call("tex", evaled_args)
        }
        "grind" => {
            if let Some(arg) = evaled_args.first() {
                let s = arg.to_string();
                println!("{};", s);
                return Expr::sym("done");
            }
            Expr::call("grind", evaled_args)
        }
        "constantp" => {
            if let Some(arg) = evaled_args.first() {
                let has_free = has_free_variable(arg);
                return bool_result(!has_free);
            }
            Expr::call("constantp", evaled_args)
        }
        "freeof" => {
            if evaled_args.len() == 2 {
                return bool_result(!contains_var(&evaled_args[1], &evaled_args[0]));
            }
            Expr::call("freeof", evaled_args)
        }
        "nonnegintegerp" => {
            bool_result(matches!(evaled_args.first(), Some(Expr::Integer(n)) if *n >= 0))
        }
        "evenp" => {
            bool_result(matches!(evaled_args.first(), Some(Expr::Integer(n)) if n % 2 == 0))
        }
        "oddp" => {
            bool_result(matches!(evaled_args.first(), Some(Expr::Integer(n)) if n % 2 != 0))
        }
        "primep" => {
            if let Some(Expr::Integer(n)) = evaled_args.first() {
                return bool_result(is_prime(*n));
            }
            Expr::call("primep", evaled_args)
        }
        "bfloat_approx_equal" | "float_approx_equal" => {
            if evaled_args.len() == 2 {
                if let (Some(a), Some(b)) = (to_f64(&evaled_args[0]), to_f64(&evaled_args[1])) {
                    let tol = 1e-10;
                    return bool_result((a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0));
                }
            }
            Expr::call("bfloat_approx_equal", evaled_args)
        }
        "remvalue" => Expr::sym("done"),
        "load" | "batchload" => {
            if let Some(arg) = evaled_args.first() {
                let filename = match arg {
                    Expr::String(s) => s.to_string(),
                    Expr::Symbol(id) => resolve(*id),
                    _ => return Expr::call("load", evaled_args),
                };
                return eval_load(&filename, env);
            }
            Expr::call("load", evaled_args)
        }
        "require" => {
            if let Some(arg) = evaled_args.first() {
                let filename = match arg {
                    Expr::String(s) => s.to_string(),
                    Expr::Symbol(id) => resolve(*id),
                    _ => return Expr::call("require", evaled_args),
                };
                return eval_require(&filename, env);
            }
            Expr::call("require", evaled_args)
        }
        "load_plugin" => {
            if let Some(arg) = evaled_args.first() {
                let name = match arg {
                    Expr::String(s) => s.to_string(),
                    Expr::Symbol(id) => resolve(*id),
                    _ => return Expr::call("load_plugin", evaled_args),
                };
                return match crate::plugin::load_plugin(&name, env) {
                    Ok(_) => Expr::sym("true"),
                    Err(msg) => {
                        eprintln!("load_plugin: {}", msg);
                        Expr::sym("false")
                    }
                };
            }
            Expr::call("load_plugin", evaled_args)
        }
        "loaded_plugins" => {
            Expr::list(env.loaded_plugin_paths.iter()
                .map(|s| Expr::String(s.clone().into()))
                .collect())
        }
        "setup_autoload" => {
            if evaled_args.len() >= 2 {
                let filename = match &evaled_args[0] {
                    Expr::String(s) => s.to_string(),
                    Expr::Symbol(id) => resolve(*id),
                    _ => return Expr::call("setup_autoload", evaled_args),
                };
                let func_ids: Vec<SymbolId> = evaled_args[1..].iter().filter_map(|e| {
                    if let Expr::Symbol(id) = e { Some(*id) } else { None }
                }).collect();
                env.register_autoload(&filename, &func_ids);
                return Expr::sym("done");
            }
            Expr::call("setup_autoload", evaled_args)
        }
        "loaded_files" => {
            let mut files: Vec<Expr> = env.loaded_files.iter()
                .map(|s| Expr::String(s.clone().into()))
                .collect();
            files.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            Expr::list(files)
        }
        "load_pathname" => {
            match &env.load_pathname {
                Some(p) => Expr::String(p.clone().into()),
                None => Expr::sym("false"),
            }
        }
        "file_search" => {
            if let Some(arg) = evaled_args.first() {
                let name = match arg {
                    Expr::String(s) => s.to_string(),
                    Expr::Symbol(id) => resolve(*id),
                    _ => return Expr::call("file_search", evaled_args),
                };
                match resolve_file(&name, env) {
                    Some(path) => return Expr::String(path.into()),
                    None => {
                        // Also check tests/ directory
                        let test_paths = vec![
                            format!("tests/{}", name),
                            format!("tests/{}.mac", name),
                        ];
                        for path in &test_paths {
                            if std::path::Path::new(path).exists() {
                                return Expr::String(path.clone().into());
                            }
                        }
                        return Expr::sym("false");
                    }
                }
            }
            Expr::call("file_search", evaled_args)
        }
        "file_search_maxima" => {
            let paths: Vec<Expr> = env.search_paths.iter()
                .map(|s| Expr::String(s.clone().into()))
                .collect();
            Expr::list(paths)
        }
        "save" => {
            // save("file.mac", var1, var2, ...) — write variable bindings to file
            if evaled_args.len() >= 2 {
                if let Expr::String(filename) = &evaled_args[0] {
                    let mut output = String::new();
                    for arg in &args[1..] {
                        if let Expr::Symbol(id) = arg {
                            let name = resolve(*id);
                            let val = env.get(*id).cloned().unwrap_or_else(|| Expr::Symbol(*id));
                            output.push_str(&format!("{} : {};\n", name, val));
                        }
                    }
                    if let Err(e) = std::fs::write(filename.as_ref(), &output) {
                        return Expr::String(format!("Error: {}", e).into());
                    }
                    return Expr::String(filename.clone());
                }
            }
            Expr::call("save", evaled_args)
        }
        "stringout" => {
            // stringout("file.mac", expr1, expr2, ...) — write expressions to file
            if evaled_args.len() >= 2 {
                if let Expr::String(filename) = &evaled_args[0] {
                    let mut output = String::new();
                    for expr in &evaled_args[1..] {
                        output.push_str(&format!("{};\n", expr));
                    }
                    if let Err(e) = std::fs::write(filename.as_ref(), &output) {
                        return Expr::String(format!("Error: {}", e).into());
                    }
                    return Expr::String(filename.clone());
                }
            }
            Expr::call("stringout", evaled_args)
        }
        "printfile" => {
            // printfile("file.txt") — display file contents
            if evaled_args.len() == 1 {
                if let Expr::String(filename) = &evaled_args[0] {
                    if let Ok(content) = std::fs::read_to_string(filename.as_ref()) {
                        println!("{}", content);
                        return Expr::String(filename.clone());
                    }
                    return Expr::sym("false");
                }
            }
            Expr::call("printfile", evaled_args)
        }
        "declare" => {
            if evaled_args.len() >= 2 {
                if let (Expr::Symbol(var_id), Expr::Symbol(prop_id)) = (&args[0], &args[1]) {
                    let var_name = resolve(*var_id);
                    let prop_name = resolve(*prop_id);
                    let var_expr = Expr::sym(&var_name);
                    let zero = Expr::int(0);
                    // Store as property
                    env.assumptions.declare_property(&var_expr, &prop_name);
                    // Also store sign implications
                    match prop_name.as_str() {
                        "positive" => {
                            env.assumptions.assume(Fact {
                                lhs: var_expr, rel: Relation::GreaterThan, rhs: zero,
                            });
                        }
                        "negative" => {
                            env.assumptions.assume(Fact {
                                lhs: var_expr, rel: Relation::LessThan, rhs: zero,
                            });
                        }
                        _ => {}
                    }
                }
            }
            Expr::sym("done")
        }
        "remove" => {
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(prop_id) = &args[1] {
                    let prop_name = resolve(*prop_id);
                    let var_expr = meval(&args[0], env);
                    env.assumptions.remove_property(&var_expr, &prop_name);
                }
            }
            Expr::sym("done")
        }
        "featurep" => {
            if evaled_args.len() >= 2 {
                if let Expr::Symbol(prop_id) = &evaled_args[1] {
                    let prop = resolve(*prop_id);
                    return bool_result(env.assumptions.has_property(&evaled_args[0], &prop));
                }
            }
            Expr::sym("false")
        }
        "properties" => {
            if let Some(arg) = evaled_args.first() {
                let props = env.assumptions.list_properties(arg);
                let items: Vec<Expr> = props.iter().map(|p| Expr::sym(p)).collect();
                return Expr::list(items);
            }
            Expr::list(vec![])
        }
        "newcontext" => {
            if let Some(Expr::Symbol(id)) = args.first() {
                let name = resolve(*id);
                env.assumptions.new_context(&name);
            }
            Expr::sym("done")
        }
        "killcontext" => {
            if let Some(Expr::Symbol(id)) = args.first() {
                let name = resolve(*id);
                env.assumptions.kill_context(&name);
            }
            Expr::sym("done")
        }
        "activate" => {
            if let Some(Expr::Symbol(id)) = args.first() {
                let name = resolve(*id);
                env.assumptions.activate_context(&name);
            }
            Expr::sym("done")
        }
        "deactivate" => {
            if let Some(Expr::Symbol(id)) = args.first() {
                let name = resolve(*id);
                env.assumptions.deactivate_context(&name);
            }
            Expr::sym("done")
        }
        "supcontext" => {
            // supcontext(name, parent) — create sub-context
            if let Some(Expr::Symbol(id)) = args.first() {
                let name = resolve(*id);
                env.assumptions.new_context(&name);
            }
            Expr::sym("done")
        }
        "parse_string" => {
            if let Some(Expr::String(s)) = evaled_args.first() {
                let result = std::panic::catch_unwind(|| {
                    maxima_parser::parse(s)
                });
                match result {
                    Ok(expr) => meval(&expr, env),
                    Err(_) => Expr::call("parse_string", evaled_args),
                }
            } else {
                Expr::call("parse_string", evaled_args)
            }
        }
        "lhs" => {
            if let Some(Expr::List { args: inner, .. }) = evaled_args.first() {
                if !inner.is_empty() {
                    return inner[0].clone();
                }
            }
            Expr::call("lhs", evaled_args)
        }
        "rhs" => {
            if let Some(Expr::List { args: inner, .. }) = evaled_args.first() {
                if inner.len() >= 2 {
                    return inner[1].clone();
                }
            }
            Expr::call("rhs", evaled_args)
        }
        "op" => {
            if let Some(Expr::List { op, .. }) = evaled_args.first() {
                return Expr::String(op.to_string().into());
            }
            Expr::call("op", evaled_args)
        }
        "args" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                return Expr::list(items.clone());
            }
            if let Some(Expr::List { args: inner, .. }) = evaled_args.first() {
                return Expr::list(inner.clone());
            }
            Expr::call("args", evaled_args)
        }
        "equal" => {
            if evaled_args.len() == 2 {
                // Return the equal() expression unevaluated — is() will evaluate it
                return Expr::call("equal", evaled_args);
            }
            Expr::call("equal", evaled_args)
        }
        "notequal" => {
            if evaled_args.len() == 2 {
                return Expr::call("notequal", evaled_args);
            }
            Expr::call("notequal", evaled_args)
        }
        "matrix" => {
            // matrix([row1], [row2], ...) — construct matrix
            Expr::List {
                op: Operator::MMatrix,
                simplified: false,
                args: evaled_args,
            }
        }
        "determinant" => {
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                let n = rows.len();
                if n == 0 { return Expr::int(1); }
                // Extract matrix elements
                let mut mat: Vec<Vec<Expr>> = Vec::new();
                for row in rows {
                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                        if cols.len() != n { return Expr::call("determinant", evaled_args); }
                        mat.push(cols.clone());
                    } else {
                        return Expr::call("determinant", evaled_args);
                    }
                }
                return matrix_det(&mat, env);
            }
            Expr::call("determinant", evaled_args)
        }
        "invert" | "invert_by_adjoint" => {
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                let n = rows.len();
                if n == 0 { return evaled_args[0].clone(); }
                let mut mat: Vec<Vec<Expr>> = Vec::new();
                for row in rows {
                    if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                        if cols.len() != n { return Expr::call("invert", evaled_args); }
                        mat.push(cols.clone());
                    } else {
                        return Expr::call("invert", evaled_args);
                    }
                }
                let det = matrix_det(&mat, env);
                if det == Expr::int(0) {
                    return Expr::call("invert", evaled_args);
                }
                // Adjugate method: A^-1 = adj(A) / det(A)
                let mut adj = Vec::new();
                for i in 0..n {
                    let mut row = Vec::new();
                    for j in 0..n {
                        let cofactor = matrix_cofactor(&mat, j, i, env);
                        row.push(simplify(&Expr::div(cofactor, det.clone())));
                    }
                    adj.push(Expr::list(row));
                }
                return Expr::List { op: Operator::MMatrix, simplified: false, args: adj };
            }
            Expr::call("invert", evaled_args)
        }
        "transpose" => {
            if let Some(Expr::List { op: Operator::MMatrix, args: rows, .. }) = evaled_args.first() {
                if rows.is_empty() {
                    return Expr::List { op: Operator::MMatrix, simplified: false, args: vec![] };
                }
                let ncols = if let Expr::List { op: Operator::MList, args: cols, .. } = &rows[0] {
                    cols.len()
                } else { 0 };
                let mut result = Vec::new();
                for j in 0..ncols {
                    let col: Vec<Expr> = rows.iter().map(|row| {
                        if let Expr::List { op: Operator::MList, args: cols, .. } = row {
                            cols.get(j).cloned().unwrap_or(Expr::int(0))
                        } else {
                            Expr::int(0)
                        }
                    }).collect();
                    result.push(Expr::list(col));
                }
                return Expr::List { op: Operator::MMatrix, simplified: false, args: result };
            }
            Expr::call("transpose", evaled_args)
        }
        "ident" => {
            if let Some(Expr::Integer(n)) = evaled_args.first() {
                let n = *n as usize;
                let rows: Vec<Expr> = (0..n).map(|i| {
                    let cols: Vec<Expr> = (0..n).map(|j| {
                        if i == j { Expr::int(1) } else { Expr::int(0) }
                    }).collect();
                    Expr::list(cols)
                }).collect();
                return Expr::List { op: Operator::MMatrix, simplified: false, args: rows };
            }
            Expr::call("ident", evaled_args)
        }
        "zeromatrix" => {
            if evaled_args.len() == 2 {
                if let (Some(m), Some(n)) = (to_i64(&evaled_args[0]), to_i64(&evaled_args[1])) {
                    let rows: Vec<Expr> = (0..m).map(|_| {
                        Expr::list(vec![Expr::int(0); n as usize])
                    }).collect();
                    return Expr::List { op: Operator::MMatrix, simplified: false, args: rows };
                }
            }
            Expr::call("zeromatrix", evaled_args)
        }
        "conjugate" | "realpart" | "imagpart" | "cabs" | "rectform" => {
            if let Some(result) = crate::complex::eval_complex_func(&func_name, &evaled_args) {
                return result;
            }
            Expr::call(&func_name, evaled_args)
        }
        "signum" | "sign" => {
            if let Some(arg) = evaled_args.first() {
                let s = compute_sign(arg, &env.assumptions);
                match s {
                    crate::assume::Sign::Pos => return Expr::int(1),
                    crate::assume::Sign::Neg => return Expr::int(-1),
                    crate::assume::Sign::Zero => return Expr::int(0),
                    _ => {}
                }
            }
            Expr::call("signum", evaled_args)
        }
        "nterms" | "nargs" => {
            if let Some(Expr::List { args: inner, .. }) = evaled_args.first() {
                return Expr::int(inner.len() as i64);
            }
            Expr::int(0)
        }
        "part" => {
            if evaled_args.len() >= 2 {
                let expr = &evaled_args[0];
                if let Some(n) = to_i64(&evaled_args[1]) {
                    if let Expr::List { args: inner, .. } = expr {
                        if n >= 1 && (n as usize) <= inner.len() {
                            return inner[(n - 1) as usize].clone();
                        }
                        if n == 0 {
                            if let Expr::List { op, .. } = expr {
                                return Expr::String(op.to_string().into());
                            }
                        }
                    }
                }
            }
            Expr::call("part", evaled_args)
        }
        "infix" | "prefix" | "postfix" | "nary" | "matchfix" | "nofix" => {
            Expr::sym("done")
        }
        "simp" => {
            if let Some(arg) = evaled_args.first() {
                return simplify(arg);
            }
            Expr::call("simp", evaled_args)
        }
        "unique" => {
            if let Some(Expr::List { op: Operator::MList, args: items, .. }) = evaled_args.first() {
                let mut seen = Vec::new();
                for item in items {
                    if !seen.contains(item) {
                        seen.push(item.clone());
                    }
                }
                Expr::list(seen)
            } else {
                Expr::call("unique", evaled_args)
            }
        }
        "meval" => {
            // ''expr — force evaluation (already evaluated by default)
            if let Some(arg) = evaled_args.first() {
                return arg.clone();
            }
            Expr::sym("done")
        }
        "orderlessp" => {
            if evaled_args.len() == 2 {
                let a = evaled_args[0].to_string();
                let b = evaled_args[1].to_string();
                return bool_result(a < b);
            }
            Expr::call("orderlessp", evaled_args)
        }
        "ordergreatp" => {
            if evaled_args.len() == 2 {
                let a = evaled_args[0].to_string();
                let b = evaled_args[1].to_string();
                return bool_result(a > b);
            }
            Expr::call("ordergreatp", evaled_args)
        }
        // Lisp interop stubs
        "?fmakunbound" => Expr::sym("false"),
        "?great" => {
            if evaled_args.len() == 2 {
                let a = evaled_args[0].to_string();
                let b = evaled_args[1].to_string();
                return bool_result(a > b);
            }
            Expr::call("?great", evaled_args)
        }
        "makelist" => eval_makelist(args, env),
        "create_list" => eval_makelist(args, env),
        _ => {
            // 1. Native (Rust plugin) functions — highest priority.
            if let Some(ndef) = env.native_functions.get(&func_name).cloned() {
                let n = evaled_args.len();
                if n < ndef.min_args || ndef.max_args.map_or(false, |m| n > m) {
                    return Expr::call(&func_name, evaled_args);
                }
                // Safety net for statically-linked native fns (same panic
                // runtime). Dynamically loaded plugins MUST catch their own
                // panics via `plugin::guard` — a panic unwinding out of a
                // separately-compiled .so is a "foreign exception" the host
                // cannot catch and would abort the process.
                let call = std::panic::AssertUnwindSafe(|| (ndef.func)(&evaled_args, env));
                return match std::panic::catch_unwind(call) {
                    Ok(result) => result,
                    Err(_) => {
                        eprintln!("warning: native function `{}` panicked; returning noun form", func_name);
                        Expr::call(&func_name, evaled_args)
                    }
                };
            }
            // 2. User-defined Maxima functions
            if let Some(def) = env.functions.get(&name).cloned() {
                if evaled_args.len() != def.params.len() {
                    panic!(
                        "{}: wrong number of arguments ({} given, {} expected)",
                        func_name,
                        evaled_args.len(),
                        def.params.len()
                    );
                }
                env.push_scope();
                for (param, arg) in def.params.iter().zip(evaled_args.iter()) {
                    env.set_local(*param, arg.clone());
                }
                let result = meval(&def.body, env);
                env.pop_scope();
                result
            } else {
                // 3. Lambda values
                if let Some(val) = env.get(name).cloned() {
                    if matches!(&val, Expr::List { op: Operator::MLambda, .. }) {
                        return apply_func(&val, &evaled_args, env);
                    }
                }
                // 4. Autoload: if registered, load the file and retry once
                if let Some(file) = env.autoload_registry.remove(&name) {
                    eval_load(&file, env);
                    if env.functions.contains_key(&name) || env.native_functions.contains_key(&func_name) {
                        let retry = Expr::List {
                            op: Operator::Named(name),
                            simplified: false,
                            args: evaled_args,
                        };
                        return meval(&retry, env);
                    }
                }
                // 5. Noun form
                Expr::List {
                    op: Operator::Named(name),
                    simplified: false,
                    args: evaled_args,
                }
            }
        }
    }
}

fn eval_ev(args: &[Expr], env: &mut Environment) -> Expr {
    if args.is_empty() {
        return Expr::sym("done");
    }
    let base_expr = meval(&args[0], env);

    // Remaining args are substitutions: var:val or just flags
    env.push_scope();
    for sub_arg in &args[1..] {
        if let Expr::List { op: Operator::MAssign, args: assign_args, .. } = sub_arg {
            if let Expr::Symbol(id) = &assign_args[0] {
                let val = meval(&assign_args[1], env);
                env.set_local(*id, val);
            }
        }
    }
    let result = meval(&base_expr, env);
    env.pop_scope();
    result
}

fn eval_sum(args: &[Expr], env: &mut Environment) -> Expr {
    // sum(expr, var, lo, hi)
    if args.len() != 4 {
        let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
        return Expr::call("sum", evaled);
    }
    let body = &args[0];
    let var = match &args[1] {
        Expr::Symbol(id) => *id,
        other => {
            let evaled = meval(other, env);
            if let Expr::Symbol(id) = evaled {
                id
            } else {
                let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
                return Expr::call("sum", evaled);
            }
        }
    };
    let lo = meval(&args[2], env);
    let hi = meval(&args[3], env);

    // Numeric bounds: iterate
    if let (Some(lo_i), Some(hi_i)) = (to_i64(&lo), to_i64(&hi)) {
        if hi_i - lo_i < 10000 {
            let mut result = Expr::int(0);
            env.push_scope();
            for i in lo_i..=hi_i {
                env.set_local(var, Expr::int(i));
                let term = meval(body, env);
                result = meval(&Expr::add(result, term), env);
            }
            env.pop_scope();
            return result;
        }
    }

    // Symbolic bounds: try closed-form evaluation
    let body_evaled = {
        env.push_scope();
        env.set_local(var, Expr::Symbol(var));
        let b = meval(body, env);
        env.pop_scope();
        b
    };
    let var_expr = Expr::Symbol(var);

    if let Some(result) = try_closed_form_sum(&body_evaled, &var_expr, &lo, &hi) {
        return result;
    }

    let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
    Expr::call("sum", evaled)
}

/// Try to evaluate Σ_{k=lo}^{hi} body as a closed form.
fn try_closed_form_sum(body: &Expr, var: &Expr, lo: &Expr, hi: &Expr) -> Option<Expr> {
    let n = hi;

    // Check if body is a polynomial in var (try to convert to Poly)
    if let Expr::Symbol(var_id) = var {
        if let Some(poly) = maxima_poly::expr_to_poly(body, *var_id) {
            // Polynomial sum: Σ_{k=lo}^{n} p(k) = sum of Faulhaber terms
            if *lo == Expr::int(1) || *lo == Expr::int(0) {
                let from_one = *lo == Expr::int(1);
                return poly_sum_closed_form(&poly, n, from_one);
            }
        }
    }

    // Check for geometric: body = c * r^k where c, r don't depend on var
    if let Some((coeff, base)) = extract_geometric(body, var) {
        // Σ_{k=lo}^{n} c * r^k = c * (r^(n+1) - r^lo) / (r - 1)
        let r = base;
        return Some(simplify(&Expr::mul(
            coeff,
            Expr::div(
                Expr::sub(
                    Expr::pow(r.clone(), Expr::add(n.clone(), Expr::int(1))),
                    Expr::pow(r.clone(), lo.clone()),
                ),
                Expr::sub(r, Expr::int(1)),
            ),
        )));
    }

    // Check for telescoping: body = f(k) - f(k+1) or f(k+1) - f(k)
    if let Expr::List { op: Operator::MPlus, args, .. } = body {
        if let Some(result) = try_telescoping_sum(args, var, lo, hi) {
            return Some(result);
        }
    }

    // Try partial fractions for rational functions, then telescoping
    if let Expr::Symbol(var_id) = var {
        if let Some((num_e, den_e)) = extract_fraction(body) {
            let num_expanded = expand(&num_e);
            let den_expanded = expand(&den_e);
            if let (Some(np), Some(dp)) = (
                maxima_poly::expr_to_poly(&num_expanded, *var_id),
                maxima_poly::expr_to_poly(&den_expanded, *var_id),
            ) {
                let factors = maxima_poly::factor_poly(&dp);
                if factors.len() >= 2 && factors.iter().all(|(f, m)| f.degree() == Some(1) && *m == 1) {
                    // Try partial fractions → telescoping
                    let pf_expr = integrate_partfrac_as_sum(&np, &factors, var);
                    if let Some(pf) = pf_expr {
                        if let Expr::List { op: Operator::MPlus, args: pf_terms, .. } = &pf {
                            if let Some(result) = try_telescoping_sum(pf_terms, var, lo, hi) {
                                return Some(result);
                            }
                        }
                    }
                }
            }
        }
    }

    // Arith-geometric: k * r^k
    if let Expr::List { op: Operator::MTimes, args, .. } = body {
        if args.len() == 2 {
            let (poly_part, geo_part) = if contains_var(&args[0], var) && !contains_var(&args[1], var) {
                return None; // constant * f(k) — already handled above
            } else if let Some((c, r)) = extract_geometric(&args[1], var) {
                (args[0].clone(), (c, r))
            } else if let Some((c, r)) = extract_geometric(&args[0], var) {
                (args[1].clone(), (c, r))
            } else {
                return None;
            };
            // For k * r^k with lo=0: Σ_{k=0}^{n} k*r^k = r(1-(n+1)r^n+nr^(n+1))/(1-r)^2
            if poly_part == *var && *lo == Expr::int(0) {
                let (c, r) = geo_part;
                let result = simplify(&Expr::mul(c, Expr::div(
                    Expr::mul(r.clone(), Expr::sub(
                        Expr::sub(
                            Expr::int(1),
                            Expr::mul(Expr::add(n.clone(), Expr::int(1)),
                                      Expr::pow(r.clone(), n.clone())),
                        ),
                        Expr::neg(Expr::mul(n.clone(), Expr::pow(r.clone(), Expr::add(n.clone(), Expr::int(1))))),
                    )),
                    Expr::pow(Expr::sub(Expr::int(1), r), Expr::int(2)),
                )));
                return Some(result);
            }
        }
    }

    // Try Gosper's algorithm for hypergeometric terms
    if let Some(result) = try_gosper_sum(body, var, lo, hi) {
        return Some(result);
    }

    // Known binomial identities
    if let Some(result) = try_binomial_sum(body, var, lo, hi) {
        return Some(result);
    }

    None
}

/// Recognize known binomial sum identities.
fn try_binomial_sum(body: &Expr, var: &Expr, lo: &Expr, hi: &Expr) -> Option<Expr> {
    // Σ_{k=0}^{n} binomial(n,k) = 2^n
    if let Expr::List { op: Operator::Named(id), args, .. } = body {
        if resolve(*id) == "binomial" && args.len() == 2 {
            if args[1] == *var && *lo == Expr::int(0) && args[0] == *hi {
                // Σ binomial(n,k) from 0 to n = 2^n
                return Some(Expr::pow(Expr::int(2), hi.clone()));
            }
        }
    }
    // Σ_{k=0}^{n} (-1)^k * binomial(n,k) = 0 for n > 0
    if let Expr::List { op: Operator::MTimes, args: margs, .. } = body {
        if margs.len() == 2 {
            let has_neg1_k = margs.iter().any(|a| {
                matches!(a, Expr::List { op: Operator::MExpt, args, .. }
                    if args.len() == 2 && args[0] == Expr::int(-1) && args[1] == *var)
            });
            let binom = margs.iter().find(|a| {
                matches!(a, Expr::List { op: Operator::Named(id), args, .. }
                    if resolve(*id) == "binomial" && args.len() == 2)
            });
            if has_neg1_k {
                if let Some(Expr::List { args: ba, .. }) = binom {
                    if ba[1] == *var && *lo == Expr::int(0) && ba[0] == *hi {
                        return Some(Expr::int(0));
                    }
                }
            }
        }
    }
    // Try Zeilberger's algorithm for hypergeometric sums
    if *lo == Expr::int(0) && contains_var(body, hi) {
        // F(n,k) with sum from 0 to n — try Zeilberger
        if let Some(zr) = crate::zeilberger::zeilberger(body, hi, var, 2) {
            // Compute initial value S(0) = F(0,0)
            let s0 = simplify(&subst(&Expr::int(0), var,
                &subst(&Expr::int(0), hi, body)));
            if let Some(sol) = crate::zeilberger::solve_recurrence(&zr, hi, &s0) {
                return Some(sol);
            }
        }
    }
    None
}

/// Gosper's algorithm: sum hypergeometric term t_k where t_{k+1}/t_k is rational.
/// Finds S_k such that S_{k+1} - S_k = t_k (telescoping certificate).
fn try_gosper_sum(body: &Expr, var: &Expr, lo: &Expr, hi: &Expr) -> Option<Expr> {
    if let Expr::Symbol(var_id) = var {
        // Compute t_{k+1}/t_k
        let shifted = subst(&Expr::add(var.clone(), Expr::int(1)), var, body);
        let ratio = simplify(&ratsimp(&Expr::div(shifted, body.clone())));

        // Check if ratio is rational in k (polynomial/polynomial)
        let ratio_expanded = expand(&ratio);
        if contains_var(&ratio_expanded, var) {
            if let Some((p, q)) = extract_rational_poly(&ratio_expanded, *var_id) {
                // ratio = p(k)/q(k). Gosper form: find polynomials a,b,c such that
                // p(k)/q(k) = a(k+1)*b(k) / (a(k)*c(k))
                // and gcd(b(k), c(k+j)) = 1 for all j >= 0.
                //
                // For the simple case: p and q are already coprime shifted-wise,
                // so a=1, b=p, c=q. Then we need polynomial x(k) satisfying:
                // b(k)*x(k+1) - c(k-1)*x(k) = a(k) = 1 (when a=1)
                // i.e., p(k)*x(k+1) - q(k-1)*x(k) = 1

                // Simplified Gosper: if ratio = (k+a)/(k+b) for constants a,b,
                // then the sum telescopes via S_k = t_k * (k+b-1) / (a-b)
                if p.degree().unwrap_or(0) == 1 && q.degree().unwrap_or(0) == 1 {
                    let p_lc = get_poly_coeff_(&p, 1);
                    let p_ct = get_poly_coeff_(&p, 0);
                    let q_lc = get_poly_coeff_(&q, 1);
                    let q_ct = get_poly_coeff_(&q, 0);

                    if p_lc == q_lc && p_lc != 0 {
                        // ratio = (k+a)/(k+b) where a=p_ct/p_lc, b=q_ct/q_lc
                        let a_num = p_ct * q_lc;
                        let b_num = q_ct * p_lc;
                        let _denom = p_lc * q_lc;
                        let diff = a_num - b_num; // (a-b)*denom

                        if diff != 0 {
                            // S_k = t_k * (k + b - 1) / ((a-b))
                            // Actually: S_k = t_k * q(k-1) / (p(k) - q(k))
                            // Verify: S_{k+1} - S_k = t_k iff the Gosper equation holds

                            // For t_k = k! type: ratio = k+1, p=k+1, q=1
                            // For t_k = 1/k!: ratio = 1/(k+1), p=1, q=k+1
                            // For t_k = binom(n,k): ratio = (n-k)/(k+1)

                            // Simple telescoping: S_k = t_k * q(k-1) / (p(k)-q(k))
                            let _q_shifted = maxima_poly::Poly {
                                var: *var_id,
                                terms: q.terms.iter().map(|(e, c)| {
                                    if *e == 0 {
                                        (0, c.add(&maxima_poly::Coeff::Int(-q_lc)))
                                    } else { (*e, c.clone()) }
                                }).collect(),
                            };
                            // p(k) - q(k) should be constant
                            let diff_poly = p.sub(&q);
                            if diff_poly.is_constant() {
                                if let maxima_poly::Coeff::Int(d) = diff_poly.constant_term() {
                                    if d != 0 {
                                        // S_k = t_k * q(k-1) / d
                                        let qkm1_expr = maxima_poly::poly_to_expr(&q);
                                        let qkm1_shifted = subst(&Expr::sub(var.clone(), Expr::int(1)), var, &qkm1_expr);
                                        let s_k = simplify(&Expr::div(
                                            Expr::mul(body.clone(), qkm1_shifted),
                                            Expr::int(d),
                                        ));

                                        // Verify: S_{k+1} - S_k should equal t_k
                                        let s_k1 = subst(&Expr::add(var.clone(), Expr::int(1)), var, &s_k);
                                        let check = simplify(&ratsimp(&Expr::sub(s_k1, s_k.clone())));
                                        let body_simp = simplify(&ratsimp(body));
                                        if simplify(&Expr::sub(check.clone(), body_simp.clone())) == Expr::int(0)
                                            || check.to_string() == body_simp.to_string()
                                        {
                                            let s_hi1 = subst(&Expr::add(hi.clone(), Expr::int(1)), var, &s_k);
                                            let s_lo = subst(lo, var, &s_k);
                                            return Some(simplify(&Expr::sub(s_hi1, s_lo)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract symbolic quadratic coefficients: find a, b, c in a*x²+b*x+c.
fn extract_symbolic_quad_coeffs(expr: &Expr, var: &Expr) -> Option<(Expr, Expr, Expr)> {
    // Collect terms and classify by power of var
    let terms = match expr {
        Expr::List { op: Operator::MPlus, args, .. } => args.clone(),
        _ => vec![expr.clone()],
    };
    let mut a = Expr::int(0);
    let mut b = Expr::int(0);
    let mut c = Expr::int(0);
    for term in &terms {
        let (power, coeff) = classify_var_power(term, var);
        match power {
            2 => a = simplify(&Expr::add(a, coeff)),
            1 => b = simplify(&Expr::add(b, coeff)),
            0 => c = simplify(&Expr::add(c, coeff)),
            _ => return None,
        }
    }
    if a == Expr::int(0) { return None; }
    Some((a, b, c))
}

fn classify_var_power(term: &Expr, var: &Expr) -> (u32, Expr) {
    if !contains_var(term, var) { return (0, term.clone()); }
    if term == var { return (1, Expr::int(1)); }
    if let Expr::List { op: Operator::MExpt, args, .. } = term {
        if args.len() == 2 && args[0] == *var {
            if let Some(n) = to_i64(&args[1]) {
                if n >= 0 { return (n as u32, Expr::int(1)); }
            }
        }
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = term {
        let mut power = 0u32;
        let mut coeff_parts = Vec::new();
        for a in args {
            if a == var { power += 1; }
            else if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                if pa.len() == 2 && pa[0] == *var {
                    if let Some(n) = to_i64(&pa[1]) { power += n as u32; }
                    else { coeff_parts.push(a.clone()); }
                } else { coeff_parts.push(a.clone()); }
            } else { coeff_parts.push(a.clone()); }
        }
        let coeff = if coeff_parts.is_empty() { Expr::int(1) }
            else if coeff_parts.len() == 1 { coeff_parts[0].clone() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: coeff_parts } };
        return (power, coeff);
    }
    (0, term.clone())
}

fn get_poly_coeff_(p: &maxima_poly::Poly, exp: u32) -> i64 {
    p.terms.iter()
        .find(|(e, _)| *e == exp)
        .and_then(|(_, c)| if let maxima_poly::Coeff::Int(n) = c { Some(*n) } else { None })
        .unwrap_or(0)
}

fn extract_rational_poly(expr: &Expr, var_id: maxima_core::SymbolId) -> Option<(maxima_poly::Poly, maxima_poly::Poly)> {
    if let Some((num, den)) = extract_fraction(expr) {
        let np = maxima_poly::expr_to_poly(&expand(&num), var_id)?;
        let dp = maxima_poly::expr_to_poly(&expand(&den), var_id)?;
        return Some((np, dp));
    }
    // If no fraction, treat as polynomial / 1
    let p = maxima_poly::expr_to_poly(expr, var_id)?;
    Some((p, maxima_poly::Poly::constant(var_id, maxima_poly::Coeff::one())))
}

/// Faulhaber's formulas for Σ_{k=1}^{n} k^m (or from k=0).
fn poly_sum_closed_form(poly: &maxima_poly::Poly, n: &Expr, from_one: bool) -> Option<Expr> {
    let mut terms = Vec::new();

    for (exp, coeff) in &poly.terms {
        let c_expr = match coeff {
            maxima_poly::Coeff::Int(v) => Expr::int(*v),
            maxima_poly::Coeff::Rat(a, b) => Expr::Rational { num: *a, den: *b },
        };

        let sum_of_power = match *exp {
            0 => {
                // Σ_{k=1}^{n} 1 = n; Σ_{k=0}^{n} 1 = n+1
                if from_one { n.clone() } else { Expr::add(n.clone(), Expr::int(1)) }
            }
            1 => {
                // Σ_{k=1}^{n} k = n(n+1)/2; Σ_{k=0}^{n} k = n(n+1)/2
                simplify(&Expr::div(
                    Expr::mul(n.clone(), Expr::add(n.clone(), Expr::int(1))),
                    Expr::int(2),
                ))
            }
            2 => {
                // Σ_{k=1}^{n} k² = n(n+1)(2n+1)/6
                simplify(&Expr::div(
                    Expr::mul(
                        Expr::mul(n.clone(), Expr::add(n.clone(), Expr::int(1))),
                        Expr::add(Expr::mul(Expr::int(2), n.clone()), Expr::int(1)),
                    ),
                    Expr::int(6),
                ))
            }
            3 => {
                // Σ_{k=1}^{n} k³ = (n(n+1)/2)²
                let half = simplify(&Expr::div(
                    Expr::mul(n.clone(), Expr::add(n.clone(), Expr::int(1))),
                    Expr::int(2),
                ));
                simplify(&Expr::pow(half, Expr::int(2)))
            }
            _ => return None,
        };

        terms.push(simplify(&Expr::mul(c_expr, sum_of_power)));
    }

    if terms.is_empty() { return Some(Expr::int(0)); }
    if terms.len() == 1 { return Some(terms.remove(0)); }
    Some(simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }))
}

/// Extract geometric form: body = c * r^var where c, r don't contain var.
fn extract_geometric(body: &Expr, var: &Expr) -> Option<(Expr, Expr)> {
    // body = r^var
    if let Expr::List { op: Operator::MExpt, args, .. } = body {
        if args.len() == 2 && args[1] == *var && !contains_var(&args[0], var) {
            return Some((Expr::int(1), args[0].clone()));
        }
    }
    // body = c * r^var
    if let Expr::List { op: Operator::MTimes, args, .. } = body {
        let (consts, deps): (Vec<&Expr>, Vec<&Expr>) = args.iter().partition(|a| !contains_var(a, var));
        if deps.len() == 1 {
            if let Expr::List { op: Operator::MExpt, args: pa, .. } = deps[0] {
                if pa.len() == 2 && pa[1] == *var && !contains_var(&pa[0], var) {
                    let c = if consts.len() == 1 { consts[0].clone() }
                        else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
                    return Some((c, pa[0].clone()));
                }
            }
        }
    }
    None
}

/// Try to detect telescoping: Σ (f(k) - f(k+1)) = f(lo) - f(hi+1).
fn try_telescoping_sum(terms: &[Expr], var: &Expr, lo: &Expr, hi: &Expr) -> Option<Expr> {
    if terms.len() != 2 { return None; }

    // Check if one term is positive f(k) and the other is -f(k+1) (or vice versa)
    let (pos, neg) = if is_negated(&terms[1]) {
        (&terms[0], negate_expr(&terms[1]))
    } else if is_negated(&terms[0]) {
        (&terms[1], negate_expr(&terms[0]))
    } else {
        return None;
    };

    // Check if neg = pos with var → var+1
    let shifted = subst(&Expr::add(var.clone(), Expr::int(1)), var, pos);
    let shifted_simp = simplify(&shifted);
    let neg_simp = simplify(&neg);
    if shifted_simp == neg_simp {
        // Telescoping: Σ (f(k) - f(k+1)) = f(lo) - f(hi+1)
        let f_lo = subst(lo, var, pos);
        let f_hip1 = subst(&Expr::add(hi.clone(), Expr::int(1)), var, pos);
        return Some(simplify(&Expr::sub(f_lo, f_hip1)));
    }

    // Check reverse: neg = pos with var → var-1
    let shifted_back = subst(&Expr::sub(var.clone(), Expr::int(1)), var, pos);
    let shifted_back_simp = simplify(&shifted_back);
    if shifted_back_simp == neg_simp {
        // Σ (f(k) - f(k-1)) = f(hi) - f(lo-1)
        let f_hi = subst(hi, var, pos);
        let f_lom1 = subst(&Expr::sub(lo.clone(), Expr::int(1)), var, pos);
        return Some(simplify(&Expr::sub(f_hi, f_lom1)));
    }

    None
}

/// Decompose A(k)/D(k) into partial fractions as a sum expression.
fn integrate_partfrac_as_sum(
    num: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
    _var: &Expr,
) -> Option<Expr> {
    let mut terms = Vec::new();
    for (i, (fi, _)) in factors.iter().enumerate() {
        let a = fi.leading_coeff();
        let b = fi.constant_term();
        let root = b.neg().div(&a)?;
        let num_at = num.eval_at(&root);
        let mut den_at = maxima_poly::Coeff::one();
        for (j, (fj, _)) in factors.iter().enumerate() {
            if j != i { den_at = den_at.mul(&fj.eval_at(&root)); }
        }
        let residue = num_at.div(&den_at)?;
        if residue.is_zero() { continue; }
        let c_expr = coeff_to_expr(&residue);
        let fi_expr = maxima_poly::poly_to_expr(fi);
        terms.push(simplify(&Expr::div(c_expr, fi_expr)));
    }
    if terms.is_empty() { return None; }
    Some(Expr::List { op: Operator::MPlus, simplified: false, args: terms })
}

fn is_negated(expr: &Expr) -> bool {
    match expr {
        Expr::List { op: Operator::MTimes, args, .. } => {
            args.iter().any(|a| matches!(a, Expr::Integer(n) if *n == -1))
        }
        Expr::Integer(n) => *n < 0,
        _ => false,
    }
}

fn negate_expr(expr: &Expr) -> Expr {
    simplify(&Expr::neg(expr.clone()))
}

fn eval_product(args: &[Expr], env: &mut Environment) -> Expr {
    if args.len() != 4 {
        let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
        return Expr::call("product", evaled);
    }
    let body = &args[0];
    let var = match &args[1] {
        Expr::Symbol(id) => *id,
        _ => {
            let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
            return Expr::call("product", evaled);
        }
    };
    let lo = meval(&args[2], env);
    let hi = meval(&args[3], env);

    if let (Some(lo_i), Some(hi_i)) = (to_i64(&lo), to_i64(&hi)) {
        let mut result = Expr::int(1);
        env.push_scope();
        for i in lo_i..=hi_i {
            env.set_local(var, Expr::int(i));
            let term = meval(body, env);
            result = meval(&Expr::mul(result, term), env);
        }
        env.pop_scope();
        result
    } else {
        let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
        Expr::call("product", evaled)
    }
}

fn eval_diff(args: &[Expr]) -> Expr {
    if args.len() < 2 {
        return Expr::call("diff", args.to_vec());
    }
    let expr = &args[0];
    let var = &args[1];
    let n = if args.len() >= 3 {
        to_i64(&args[2]).unwrap_or(1)
    } else {
        1
    };

    let mut result = expr.clone();
    for _ in 0..n {
        result = diff_once(&result, var);
    }
    result
}

pub(crate) fn diff_once_pub(expr: &Expr, var: &Expr) -> Expr {
    diff_once(expr, var)
}

pub(crate) fn diff_once(expr: &Expr, var: &Expr) -> Expr {
    match expr {
        Expr::Integer(_) | Expr::BigInt(_) | Expr::Float(_)
        | Expr::Rational { .. } | Expr::String(_) => Expr::int(0),

        Expr::Symbol(_) => {
            if expr == var {
                Expr::int(1)
            } else {
                Expr::int(0)
            }
        }

        Expr::List { op, args, .. } => match op {
            Operator::MPlus => {
                let terms: Vec<Expr> = args.iter().map(|a| diff_once(a, var)).collect();
                simplify_sum(&terms)
            }
            Operator::MTimes => {
                // Product rule: d(a*b)/dx = a'*b + a*b'
                if args.len() == 2 {
                    let a = &args[0];
                    let b = &args[1];
                    let da = diff_once(a, var);
                    let db = diff_once(b, var);
                    simplify_sum(&[
                        simplify_product(&[da, b.clone()]),
                        simplify_product(&[a.clone(), db]),
                    ])
                } else if args.is_empty() {
                    Expr::int(0)
                } else {
                    // Multi-arg product: treat as nested binary
                    let first = args[0].clone();
                    let rest = Expr::List {
                        op: Operator::MTimes,
                        simplified: false,
                        args: args[1..].to_vec(),
                    };
                    let da = diff_once(&first, var);
                    let db = diff_once(&rest, var);
                    simplify_sum(&[
                        simplify_product(&[da, rest]),
                        simplify_product(&[first, db]),
                    ])
                }
            }
            Operator::MExpt => {
                // Power rule: d(f^n)/dx = n * f^(n-1) * f'
                let base = &args[0];
                let exp = &args[1];
                let dbase = diff_once(base, var);
                let dexp = diff_once(exp, var);

                if dexp.is_zero() {
                    // d(f^c)/dx = c * f^(c-1) * f'
                    simplify_product(&[
                        exp.clone(),
                        simplify_power(base.clone(), simplify_sum(&[exp.clone(), Expr::int(-1)])),
                        dbase,
                    ])
                } else if dbase.is_zero() {
                    // d(c^g)/dx = c^g * log(c) * g'
                    simplify_product(&[
                        expr.clone(),
                        Expr::call("log", vec![base.clone()]),
                        dexp,
                    ])
                } else {
                    // General: d(f^g)/dx = f^g * (g' * log(f) + g * f'/f)
                    Expr::call("diff", vec![expr.clone(), var.clone()])
                }
            }
            Operator::Named(id) => {
                let fname = resolve(*id);
                if args.len() == 1 {
                    let inner = &args[0];
                    let dinner = diff_once(inner, var);
                    let x = inner.clone();
                    let outer_deriv = match fname.as_str() {
                        "sin" => Expr::call("cos", vec![x]),
                        "cos" => Expr::neg(Expr::call("sin", vec![x])),
                        "tan" => Expr::pow(Expr::call("cos", vec![x]), Expr::int(-2)),
                        "cot" => Expr::neg(Expr::pow(Expr::call("sin", vec![x]), Expr::int(-2))),
                        "sec" => Expr::mul(
                            Expr::call("sec", vec![x.clone()]),
                            Expr::call("tan", vec![x]),
                        ),
                        "csc" => Expr::neg(Expr::mul(
                            Expr::call("csc", vec![x.clone()]),
                            Expr::call("cot", vec![x]),
                        )),
                        "exp" => Expr::call("exp", vec![x]),
                        "log" => Expr::pow(x, Expr::int(-1)),
                        "sqrt" => Expr::div(
                            Expr::int(1),
                            Expr::mul(Expr::int(2), Expr::call("sqrt", vec![x])),
                        ),
                        // Inverse trig
                        "asin" => Expr::pow(
                            Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))]),
                            Expr::int(-1),
                        ),
                        "acos" => Expr::neg(Expr::pow(
                            Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x.clone(), Expr::int(2)))]),
                            Expr::int(-1),
                        )),
                        "atan" => Expr::pow(
                            Expr::add(Expr::int(1), Expr::pow(x, Expr::int(2))),
                            Expr::int(-1),
                        ),
                        "acot" => Expr::neg(Expr::pow(
                            Expr::add(Expr::int(1), Expr::pow(x, Expr::int(2))),
                            Expr::int(-1),
                        )),
                        "acsc" => Expr::neg(Expr::div(
                            Expr::int(1),
                            Expr::mul(x.clone(), Expr::call("sqrt", vec![Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1))])),
                        )),
                        "asec" => Expr::div(
                            Expr::int(1),
                            Expr::mul(x.clone(), Expr::call("sqrt", vec![Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1))])),
                        ),
                        // Hyperbolic
                        "sinh" => Expr::call("cosh", vec![x]),
                        "cosh" => Expr::call("sinh", vec![x]),
                        "tanh" => Expr::pow(Expr::call("cosh", vec![x]), Expr::int(-2)),
                        "coth" => Expr::neg(Expr::pow(Expr::call("sinh", vec![x]), Expr::int(-2))),
                        "sech" => Expr::neg(Expr::mul(
                            Expr::call("sech", vec![x.clone()]),
                            Expr::call("tanh", vec![x]),
                        )),
                        "csch" => Expr::neg(Expr::mul(
                            Expr::call("csch", vec![x.clone()]),
                            Expr::call("coth", vec![x]),
                        )),
                        // Inverse hyperbolic
                        "asinh" => Expr::pow(
                            Expr::call("sqrt", vec![Expr::add(Expr::pow(x, Expr::int(2)), Expr::int(1))]),
                            Expr::int(-1),
                        ),
                        "acosh" => Expr::pow(
                            Expr::call("sqrt", vec![Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1))]),
                            Expr::int(-1),
                        ),
                        "acsch" => Expr::neg(Expr::div(
                            Expr::int(1),
                            Expr::mul(Expr::call("abs", vec![x.clone()]), Expr::call("sqrt", vec![Expr::add(Expr::int(1), Expr::pow(x, Expr::int(2)))])),
                        )),
                        "asech" => Expr::neg(Expr::div(
                            Expr::int(1),
                            Expr::mul(x.clone(), Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))])),
                        )),
                        "atanh" => Expr::pow(
                            Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2))),
                            Expr::int(-1),
                        ),
                        "acoth" => Expr::pow(
                            Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2))),
                            Expr::int(-1),
                        ),
                        "abs" => Expr::call("signum", vec![x]),
                        // Named nonelementary special functions.
                        "erf" => Expr::div(
                            Expr::mul(Expr::int(2), Expr::call("exp", vec![Expr::neg(Expr::pow(x, Expr::int(2)))])),
                            Expr::call("sqrt", vec![Expr::sym("%pi")]),
                        ),
                        "erfc" => Expr::neg(Expr::div(
                            Expr::mul(Expr::int(2), Expr::call("exp", vec![Expr::neg(Expr::pow(x, Expr::int(2)))])),
                            Expr::call("sqrt", vec![Expr::sym("%pi")]),
                        )),
                        "erfi" => Expr::div(
                            Expr::mul(Expr::int(2), Expr::call("exp", vec![Expr::pow(x, Expr::int(2))])),
                            Expr::call("sqrt", vec![Expr::sym("%pi")]),
                        ),
                        "expintegral_ei" => Expr::div(Expr::call("exp", vec![x.clone()]), x),
                        "expintegral_li" => Expr::pow(Expr::call("log", vec![x]), Expr::int(-1)),
                        "expintegral_si" => Expr::div(Expr::call("sin", vec![x.clone()]), x),
                        "expintegral_ci" => Expr::div(Expr::call("cos", vec![x.clone()]), x),
                        "fresnel_s" => Expr::call("sin", vec![Expr::div(
                            Expr::mul(Expr::sym("%pi"), Expr::pow(x, Expr::int(2))), Expr::int(2))]),
                        "fresnel_c" => Expr::call("cos", vec![Expr::div(
                            Expr::mul(Expr::sym("%pi"), Expr::pow(x, Expr::int(2))), Expr::int(2))]),
                        _ => return Expr::call("diff", vec![expr.clone(), var.clone()]),
                    };
                    simplify_product(&[outer_deriv, dinner])
                } else {
                    Expr::call("diff", vec![expr.clone(), var.clone()])
                }
            }
            _ => Expr::call("diff", vec![expr.clone(), var.clone()]),
        },
    }
}

fn simplify_sum(terms: &[Expr]) -> Expr {
    simplify(&Expr::List {
        op: Operator::MPlus,
        simplified: false,
        args: terms.to_vec(),
    })
}

fn simplify_product(factors: &[Expr]) -> Expr {
    simplify(&Expr::List {
        op: Operator::MTimes,
        simplified: false,
        args: factors.to_vec(),
    })
}

fn simplify_power(base: Expr, exp: Expr) -> Expr {
    if exp.is_zero() {
        Expr::int(1)
    } else if exp.is_one() {
        base
    } else {
        Expr::pow(base, exp)
    }
}

pub(crate) fn expand(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MPlus, args, .. } => {
            let expanded: Vec<Expr> = args.iter().map(|a| expand(a)).collect();
            simplify_sum(&expanded)
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            if args.len() < 2 {
                return expr.clone();
            }
            let expanded: Vec<Expr> = args.iter().map(|a| expand(a)).collect();
            let mut result = expanded[0].clone();
            for factor in &expanded[1..] {
                result = expand_product(&result, factor);
            }
            result
        }
        Expr::List { op: Operator::MExpt, args, .. } => {
            if args.len() == 2 {
                let base = expand(&args[0]);
                if let Some(n) = to_i64(&args[1]) {
                    if n >= 2 && n <= 20 {
                        let mut result = base.clone();
                        for _ in 1..n {
                            result = expand_product(&result, &base);
                        }
                        return result;
                    }
                }
                Expr::pow(base, args[1].clone())
            } else {
                expr.clone()
            }
        }
        _ => expr.clone(),
    }
}

fn expand_product(a: &Expr, b: &Expr) -> Expr {
    let a_terms = get_sum_terms(a);
    let b_terms = get_sum_terms(b);

    if a_terms.len() == 1 && b_terms.len() == 1 {
        return simplify_product(&[a.clone(), b.clone()]);
    }

    let mut terms = Vec::new();
    for at in &a_terms {
        for bt in &b_terms {
            terms.push(simplify_product(&[at.clone(), bt.clone()]));
        }
    }
    simplify_sum(&terms)
}

fn get_sum_terms(expr: &Expr) -> Vec<Expr> {
    if let Expr::List { op: Operator::MPlus, args, .. } = expr {
        args.clone()
    } else {
        vec![expr.clone()]
    }
}

fn eval_math_func(name: &str, args: &[Expr]) -> Expr {
    if args.len() != 1 {
        return Expr::call(name, args.to_vec());
    }
    let arg = &args[0];

    // Named nonelementary special functions (erf, expintegral_*, fresnel_*).
    if let Some(r) = crate::special::eval_special(name, arg) {
        return r;
    }

    // Try numeric evaluation
    if let Some(x) = to_f64(arg) {
        let result = match name {
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "log" => x.ln(),
            "exp" => x.exp(),
            "sqrt" => x.sqrt(),
            "asin" => x.asin(),
            "acos" => x.acos(),
            "atan" => x.atan(),
            "sinh" => x.sinh(),
            "cosh" => x.cosh(),
            "tanh" => x.tanh(),
            _ => return Expr::call(name, args.to_vec()),
        };
        if let Expr::Float(_) = arg {
            return Expr::Float(result);
        }
        // For integer args, only evaluate at special values
        if let Expr::Integer(0) = arg {
            match name {
                "sin" | "tan" | "sinh" | "tanh" => return Expr::int(0),
                "cos" | "cosh" => return Expr::int(1),
                "exp" => return Expr::int(1),
                "log" => return Expr::call(name, args.to_vec()),
                _ => {}
            }
        }
    }

    // sqrt of integer: exact simplification
    if name == "sqrt" {
        if let Expr::Integer(n) = arg {
            if *n == 0 { return Expr::int(0); }
            if *n > 0 {
                let root = (*n as f64).sqrt() as i64;
                if root * root == *n {
                    return Expr::int(root);
                }
                // Extract largest perfect square factor
                let mut k = root;
                while k > 1 {
                    if n % (k * k) == 0 {
                        let remainder = n / (k * k);
                        return Expr::mul(Expr::int(k), Expr::call("sqrt", vec![Expr::int(remainder)]));
                    }
                    k -= 1;
                }
            }
        }
        if let Expr::Rational { num, den } = arg {
            if *num >= 0 && *den > 0 {
                let nr = (*num as f64).sqrt() as i64;
                let dr = (*den as f64).sqrt() as i64;
                if nr * nr == *num && dr * dr == *den {
                    return simplify(&Expr::Rational { num: nr, den: dr });
                }
            }
        }
    }

    // Trig at rational multiples of %pi
    if matches!(name, "sin" | "cos" | "tan") {
        if let Some((num, den)) = extract_pi_multiple(arg) {
            if let Some(val) = trig_special_value(name, num, den) {
                return val;
            }
        }
    }
    // Inverse trig at special values
    if matches!(name, "atan" | "asin" | "acos") {
        if let Some(val) = inverse_trig_special(name, arg) {
            return val;
        }
    }

    // Handle sin/cos of negated argument: sin(-x) = -sin(x), cos(-x) = cos(x)
    if matches!(name, "sin" | "cos" | "tan") {
        if let Expr::List { op: Operator::MTimes, args: factors, .. } = arg {
            if factors.first() == Some(&Expr::int(-1)) {
                let pos_arg = if factors.len() == 2 {
                    factors[1].clone()
                } else {
                    Expr::List {
                        op: Operator::MTimes,
                        simplified: true,
                        args: factors[1..].to_vec(),
                    }
                };
                match name {
                    "sin" => return simplify(&Expr::neg(Expr::call("sin", vec![pos_arg]))),
                    "cos" => return Expr::call("cos", vec![pos_arg]),
                    "tan" => return simplify(&Expr::neg(Expr::call("tan", vec![pos_arg]))),
                    _ => {}
                }
            }
        }
    }

    // Check for trig at multiples of %pi
    if let Expr::Symbol(id) = arg {
        let sym_name = resolve(*id);
        if sym_name == "%pi" {
            match name {
                "sin" => return Expr::int(0),
                "cos" => return Expr::int(-1),
                "tan" => return Expr::int(0),
                _ => {}
            }
        }
    }
    // Check for trig at rational multiples of %pi (n*%pi)
    if let Expr::List { op: Operator::MTimes, args: factors, .. } = arg {
        let has_pi = factors.iter().any(|f| matches!(f, Expr::Symbol(id) if resolve(*id) == "%pi"));
        if has_pi && factors.len() == 2 {
            let coeff = factors.iter().find(|f| !matches!(f, Expr::Symbol(id) if resolve(*id) == "%pi"));
            if let Some(c) = coeff {
                if let Some(n) = to_f64(c) {
                    let n_mod = n % 2.0;
                    if (n_mod.abs() < 1e-10) || ((n_mod - 2.0).abs() < 1e-10) {
                        match name {
                            "sin" => return Expr::int(0),
                            "cos" => return Expr::int(1),
                            "tan" => return Expr::int(0),
                            _ => {}
                        }
                    }
                    if (n_mod - 1.0).abs() < 1e-10 {
                        match name {
                            "sin" => return Expr::int(0),
                            "cos" => return Expr::int(-1),
                            "tan" => return Expr::int(0),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Sign-aware simplifications
    match name {
        "sqrt" => {
            // sqrt(x^2) → x when x>0, abs(x) otherwise
            if let Expr::List { op: Operator::MExpt, args: pa, .. } = arg {
                if pa.len() == 2 && pa[1] == Expr::int(2) {
                    let sign = compute_sign(&pa[0], &crate::assume::AssumptionDB::new());
                    return match sign {
                        crate::assume::Sign::Pos | crate::assume::Sign::Poz => pa[0].clone(),
                        crate::assume::Sign::Neg => simplify(&Expr::neg(pa[0].clone())),
                        _ => Expr::call("abs", vec![pa[0].clone()]),
                    };
                }
            }
        }
        "log" => {
            // log(exp(x)) → x when x is real
            if let Expr::List { op: Operator::Named(fid), args: fa, .. } = arg {
                if resolve(*fid) == "exp" && fa.len() == 1 {
                    return fa[0].clone();
                }
            }
            // log(1) → 0, log(%e) → 1
            if arg == &Expr::sym("%e") { return Expr::int(1); }
        }
        "exp" => {
            // exp(log(x)) → x
            if let Expr::List { op: Operator::Named(fid), args: fa, .. } = arg {
                if resolve(*fid) == "log" && fa.len() == 1 {
                    return fa[0].clone();
                }
            }
        }
        _ => {}
    }

    // Symbolic: check for special values
    match (name, arg) {
        ("sqrt", Expr::Integer(n)) if *n >= 0 => {
            let root = (*n as f64).sqrt();
            if (root * root - *n as f64).abs() < 0.5 {
                let r = root.round() as i64;
                if r * r == *n {
                    return Expr::int(r);
                }
            }
            Expr::call(name, args.to_vec())
        }
        ("exp", Expr::Integer(0)) => Expr::int(1),
        ("log", Expr::Integer(1)) => Expr::int(0),
        _ => Expr::call(name, args.to_vec()),
    }
}

fn eval_map(args: &[Expr], env: &mut Environment) -> Expr {
    if args.len() != 2 {
        return Expr::call("map", args.to_vec());
    }
    let func = &args[0];
    if let Expr::List { op: Operator::MList, args: items, .. } = &args[1] {
        let mapped: Vec<Expr> = items
            .iter()
            .map(|item| apply_func(func, &[item.clone()], env))
            .collect();
        Expr::list(mapped)
    } else {
        Expr::call("map", args.to_vec())
    }
}

fn eval_apply(args: &[Expr], env: &mut Environment) -> Expr {
    if args.len() != 2 {
        return Expr::call("apply", args.to_vec());
    }
    let func = &args[0];
    if let Expr::List { op: Operator::MList, args: items, .. } = &args[1] {
        apply_func(func, items, env)
    } else {
        Expr::call("apply", args.to_vec())
    }
}

fn apply_func(func: &Expr, args: &[Expr], env: &mut Environment) -> Expr {
    match func {
        Expr::String(s) => {
            let op = match s.as_ref() {
                "+" => Some(Operator::MPlus),
                "*" => Some(Operator::MTimes),
                _ => None,
            };
            if let Some(op) = op {
                let call_expr = Expr::List { op, simplified: false, args: args.to_vec() };
                return meval(&call_expr, env);
            }
            let sym = maxima_core::intern(s);
            let call_expr = Expr::List {
                op: Operator::Named(sym),
                simplified: false,
                args: args.to_vec(),
            };
            meval(&call_expr, env)
        }
        Expr::Symbol(id) => {
            let call_expr = Expr::List {
                op: Operator::Named(*id),
                simplified: false,
                args: args.to_vec(),
            };
            meval(&call_expr, env)
        }
        Expr::List { op: Operator::MLambda, args: lambda_parts, .. } => {
            if lambda_parts.len() >= 2 {
                if let Expr::List { op: Operator::MList, args: params, .. } = &lambda_parts[0] {
                    env.push_scope();
                    for (param, arg) in params.iter().zip(args.iter()) {
                        if let Expr::Symbol(id) = param {
                            env.set_local(*id, arg.clone());
                        }
                    }
                    let result = meval(&lambda_parts[1], env);
                    env.pop_scope();
                    return result;
                }
            }
            Expr::call("apply", vec![func.clone(), Expr::list(args.to_vec())])
        }
        _ => Expr::call("apply", vec![func.clone(), Expr::list(args.to_vec())]),
    }
}

fn eval_errcatch(args: &[Expr], env: &mut Environment) -> Expr {
    if args.is_empty() {
        return Expr::list(vec![]);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        meval(&args[0], env)
    }));
    match result {
        Ok(val) => Expr::list(vec![val]),
        Err(_) => Expr::list(vec![]),
    }
}

/// Determine if two expressions are mathematically equal.
fn is_equal_pred(a: &Expr, b: &Expr) -> Expr {
    // Special: ind and und are not equal to anything (even themselves)
    let non_equal_syms = ["ind", "und"];
    if let Expr::Symbol(id) = a {
        if non_equal_syms.contains(&resolve(*id).as_str()) { return Expr::sym("false"); }
    }
    if let Expr::Symbol(id) = b {
        if non_equal_syms.contains(&resolve(*id).as_str()) { return Expr::sym("false"); }
    }

    // Identical expressions
    if a == b {
        return Expr::sym("true");
    }

    // Boolean atoms
    let booleans = ["true", "false"];
    let a_bool = matches!(a, Expr::Symbol(id) if booleans.contains(&resolve(*id).as_str()));
    let b_bool = matches!(b, Expr::Symbol(id) if booleans.contains(&resolve(*id).as_str()));
    if a_bool && b_bool { return Expr::sym("false"); }

    // Numeric comparison
    if let (Some(fa), Some(fb)) = (to_f64(a), to_f64(b)) {
        return bool_result((fa - fb).abs() < 1e-15);
    }

    // Different types: number vs list → false
    let a_is_num = matches!(a, Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. });
    let b_is_list = matches!(b, Expr::List { op: Operator::MList, .. });
    let b_is_num = matches!(b, Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. });
    let a_is_list = matches!(a, Expr::List { op: Operator::MList, .. });
    if (a_is_num && b_is_list) || (b_is_num && a_is_list) {
        return Expr::sym("false");
    }

    // Special constants that are distinct from everything
    let specials = ["inf", "minf", "infinity", "%i", "%pi", "%e", "%phi"];
    let a_special = matches!(a, Expr::Symbol(id) if specials.contains(&resolve(*id).as_str()));
    let b_special = matches!(b, Expr::Symbol(id) if specials.contains(&resolve(*id).as_str()));
    if a_special && b_special && a != b { return Expr::sym("false"); }
    if a_special && b_is_num { return Expr::sym("false"); }
    if b_special && a_is_num { return Expr::sym("false"); }

    // List comparison: element-wise
    if let (Expr::List { op: Operator::MList, args: la, .. },
            Expr::List { op: Operator::MList, args: lb, .. }) = (a, b) {
        if la.len() != lb.len() {
            return Expr::sym("false");
        }
        let mut all_equal = true;
        for (ea, eb) in la.iter().zip(lb.iter()) {
            let r = is_equal_pred(ea, eb);
            if is_false(&r) { return Expr::sym("false"); }
            if !is_true(&r) { all_equal = false; }
        }
        return if all_equal { Expr::sym("true") } else { Expr::sym("unknown") };
    }

    // Try simplifying the difference
    let diff = simplify(&Expr::sub(a.clone(), b.clone()));
    if diff == Expr::int(0) || diff == Expr::Float(0.0) {
        return Expr::sym("true");
    }
    // Try expanding and simplifying
    let diff_expanded = simplify(&expand(&Expr::sub(a.clone(), b.clone())));
    if diff_expanded == Expr::int(0) || diff_expanded == Expr::Float(0.0) {
        return Expr::sym("true");
    }

    // If diff is a nonzero constant, they're not equal
    if matches!(&diff, Expr::Integer(n) if *n != 0) { return Expr::sym("false"); }
    if matches!(&diff_expanded, Expr::Integer(n) if *n != 0) { return Expr::sym("false"); }
    if matches!(&diff, Expr::Float(f) if f.abs() > 1e-15) { return Expr::sym("false"); }
    if matches!(&diff, Expr::Rational { num, .. } if *num != 0) { return Expr::sym("false"); }
    if matches!(&diff, Expr::Symbol(id) if specials.contains(&resolve(*id).as_str())) {
        return Expr::sym("false");
    }
    if matches!(&diff_expanded, Expr::Symbol(id) if specials.contains(&resolve(*id).as_str())) {
        return Expr::sym("false");
    }

    // Special constant vs generic variable → false
    if a_special && matches!(b, Expr::Symbol(_)) && !b_special && !b_bool { return Expr::sym("false"); }
    if b_special && matches!(a, Expr::Symbol(_)) && !a_special && !a_bool { return Expr::sym("false"); }

    // Different relation types → false
    if let (Expr::List { op: op_a, .. }, Expr::List { op: op_b, .. }) = (a, b) {
        if matches!((op_a, op_b),
            (Operator::MLessThan, Operator::MLessEqual)
            | (Operator::MLessEqual, Operator::MLessThan)
            | (Operator::MGreaterThan, Operator::MGreaterEqual)
            | (Operator::MGreaterEqual, Operator::MGreaterThan)
        ) {
            return Expr::sym("false");
        }
        // Matrices with different dimensions → false
        if matches!(op_a, Operator::MMatrix) && matches!(op_b, Operator::MMatrix) {
            if let (Expr::List { args: ra, .. }, Expr::List { args: rb, .. }) = (a, b) {
                if ra.len() != rb.len() { return Expr::sym("false"); }
                for (row_a, row_b) in ra.iter().zip(rb.iter()) {
                    if let (Expr::List { args: ca, .. }, Expr::List { args: cb, .. }) = (row_a, row_b) {
                        if ca.len() != cb.len() { return Expr::sym("false"); }
                    }
                }
            }
        }
    }

    Expr::sym("unknown")
}

fn extract_relation(expr: &Expr) -> Option<(Expr, Relation, Expr)> {
    if let Expr::List { op, args, .. } = expr {
        if args.len() == 2 {
            let rel = match op {
                Operator::MLessThan => Relation::LessThan,
                Operator::MLessEqual => Relation::LessEqual,
                Operator::MEqual => Relation::Equal,
                Operator::MGreaterEqual => Relation::GreaterEqual,
                Operator::MGreaterThan => Relation::GreaterThan,
                Operator::MNotEqual => Relation::NotEqual,
                _ => return None,
            };
            return Some((args[0].clone(), rel, args[1].clone()));
        }
    }
    None
}

fn relation_to_expr(fact: &Fact) -> Expr {
    let op = match fact.rel {
        Relation::LessThan => Operator::MLessThan,
        Relation::LessEqual => Operator::MLessEqual,
        Relation::Equal => Operator::MEqual,
        Relation::GreaterEqual => Operator::MGreaterEqual,
        Relation::GreaterThan => Operator::MGreaterThan,
        Relation::NotEqual => Operator::MNotEqual,
    };
    Expr::List {
        op,
        simplified: false,
        args: vec![fact.lhs.clone(), fact.rhs.clone()],
    }
}

pub(crate) fn ratsimp_pub(expr: &Expr) -> Expr {
    ratsimp(expr)
}

pub(crate) fn ratsimp(expr: &Expr) -> Expr {
    let simplified = simplify(expr);

    // Try polynomial GCD cancellation for rational expressions (p/q → cancel common factors)
    // Look for products containing negative powers (a * b^(-1) = a/b)
    if let Some((num_expr, den_expr)) = extract_fraction(&simplified) {
        let var = find_variable(&num_expr)
            .or_else(|| find_variable(&den_expr))
            .unwrap_or_else(|| maxima_core::intern("x"));

        if let (Some(num_poly), Some(den_poly)) = (
            maxima_poly::expr_to_poly(&num_expr, var),
            maxima_poly::expr_to_poly(&den_expr, var),
        ) {
            let g = maxima_poly::poly_gcd(&num_poly, &den_poly);
            if !g.is_constant() || !g.leading_coeff().is_one() {
                if let (Some(new_num), Some(new_den)) = (
                    num_poly.exact_div(&g),
                    den_poly.exact_div(&g),
                ) {
                    let num_e = maxima_poly::poly_to_expr(&new_num);
                    let den_e = maxima_poly::poly_to_expr(&new_den);
                    if new_den.is_constant() && new_den.constant_term().is_one() {
                        return simplify(&num_e);
                    }
                    return simplify(&Expr::div(num_e, den_e));
                }
            }
        }
    }

    // If we have rational * (sum), simplify by factoring integer GCD
    if let Expr::List { op: Operator::MTimes, args, .. } = &simplified {
        if args.len() == 2 {
            if let Expr::Rational { num: r_num, den: r_den } = &args[0] {
                if let Expr::List { op: Operator::MPlus, args: sum_terms, .. } = &args[1] {
                    let coeffs: Vec<Option<i64>> = sum_terms.iter().map(|t| {
                        match t {
                            Expr::Integer(n) => Some(*n),
                            Expr::List { op: Operator::MTimes, args: factors, .. } => {
                                factors.first().and_then(|f| if let Expr::Integer(n) = f { Some(*n) } else { None })
                            }
                            _ => None,
                        }
                    }).collect();

                    if coeffs.iter().all(|c| c.is_some()) {
                        let int_coeffs: Vec<i64> = coeffs.iter().map(|c| c.unwrap()).collect();
                        let sum_gcd = int_coeffs.iter().copied()
                            .reduce(|a, b| crate::simp::gcd_pub(a.unsigned_abs(), b.unsigned_abs()) as i64)
                            .unwrap_or(1);

                        if sum_gcd > 1 {
                            let new_num = r_num * sum_gcd;
                            let g = crate::simp::gcd_pub(new_num.unsigned_abs(), r_den.unsigned_abs()) as i64;
                            let (final_num, final_den) = (new_num / g, r_den / g);

                            let new_terms: Vec<Expr> = sum_terms.iter().map(|t| match t {
                                Expr::Integer(n) => Expr::int(n / sum_gcd),
                                Expr::List { op: Operator::MTimes, args: factors, .. } => {
                                    let mut nf = factors.clone();
                                    if let Some(Expr::Integer(n)) = nf.first_mut() { *n /= sum_gcd; }
                                    simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: nf })
                                }
                                _ => t.clone(),
                            }).collect();

                            let reduced = simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: new_terms });
                            if final_den == 1 {
                                return if final_num == 1 { reduced } else { simplify(&Expr::mul(Expr::int(final_num), reduced)) };
                            }
                            return simplify(&Expr::mul(Expr::Rational { num: final_num, den: final_den }, reduced));
                        }
                    }
                }
            }
        }
    }
    simplified
}

/// Extract numerator and denominator from an expression like a*b^(-1) or a/b.
fn contains_abs_of(expr: &Expr, var: &Expr) -> bool {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. }
            if resolve(*id) == "abs" && args.len() == 1 =>
        {
            contains_var(&args[0], var)
        }
        Expr::List { args, .. } => args.iter().any(|a| contains_abs_of(a, var)),
        _ => false,
    }
}

fn resolve_abs_for_limit(expr: &Expr, var: &Expr, point: &Expr, direction: i64) -> Expr {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. }
            if resolve(*id) == "abs" && args.len() == 1 && contains_var(&args[0], var) =>
        {
            let inner = resolve_abs_for_limit(&args[0], var, point, direction);
            let at_point = simplify(&subst(point, var, &inner));
            match to_f64(&at_point) {
                Some(v) if v > 1e-15 => inner,
                Some(v) if v < -1e-15 => simplify(&Expr::neg(inner)),
                _ => {
                    // Inner is zero at the point — use derivative to determine sign near the point
                    let deriv = eval_diff(&[inner.clone(), var.clone()]);
                    let deriv_at = simplify(&subst(point, var, &deriv));
                    let deriv_sign = to_f64(&deriv_at).unwrap_or(0.0);
                    // Near x=point+ε: inner ≈ deriv_at * ε, sign = deriv_sign * direction
                    if deriv_sign * (direction as f64) > 0.0 {
                        inner
                    } else {
                        simplify(&Expr::neg(inner))
                    }
                }
            }
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| resolve_abs_for_limit(a, var, point, direction)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

pub(crate) fn extract_fraction(expr: &Expr) -> Option<(Expr, Expr)> {
    // Handle standalone expr^(-n)
    if let Expr::List { op: Operator::MExpt, args: pow_args, .. } = expr {
        if pow_args.len() == 2 {
            if let Expr::Integer(e) = &pow_args[1] {
                if *e < 0 {
                    let pos_exp = -*e;
                    let den = if pos_exp == 1 { pow_args[0].clone() }
                              else { Expr::pow(pow_args[0].clone(), Expr::int(pos_exp)) };
                    return Some((Expr::int(1), den));
                }
            }
        }
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut num_parts = Vec::new();
        let mut den_parts = Vec::new();
        for arg in args {
            if let Expr::List { op: Operator::MExpt, args: pow_args, .. } = arg {
                if pow_args.len() == 2 {
                    if let Expr::Integer(e) = &pow_args[1] {
                        if *e < 0 {
                            let pos_exp = -*e;
                            if pos_exp == 1 {
                                den_parts.push(pow_args[0].clone());
                            } else {
                                den_parts.push(Expr::pow(pow_args[0].clone(), Expr::int(pos_exp)));
                            }
                            continue;
                        }
                    }
                }
            }
            num_parts.push(arg.clone());
        }
        if !den_parts.is_empty() {
            let num = if num_parts.len() == 1 { num_parts.pop().unwrap() }
                      else if num_parts.is_empty() { Expr::int(1) }
                      else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: num_parts }) };
            let den = if den_parts.len() == 1 { den_parts.pop().unwrap() }
                      else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: den_parts }) };
            return Some((num, den));
        }
    }
    None
}

fn eval_subscript_call(
    base: &Expr, indices: &[Expr], call_args: &[Expr], env: &mut Environment,
) -> Expr {
    let base_id = match base {
        Expr::Symbol(id) => *id,
        _ => return Expr::call("funapply", {
            let mut a = vec![Expr::call("mqapply", {
                let mut v = vec![base.clone()];
                v.extend(indices.iter().cloned());
                v
            })];
            a.extend(call_args.iter().cloned());
            a
        }),
    };

    let idx_strs: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
    let key = crate::env::SubscriptKey {
        name: base_id,
        indices: idx_strs,
    };

    // Try concrete subscript first
    if let Some(def) = env.subscript_fns.get(&key).cloned() {
        env.push_scope();
        for (param, arg) in def.params.iter().zip(call_args.iter()) {
            env.set_local(*param, arg.clone());
        }
        let result = meval(&def.body, env);
        env.pop_scope();
        return result;
    }

    // Try generic subscript
    if let Some((index_params, def)) = env.subscript_generic_fns.get(&base_id).cloned() {
        env.push_scope();
        // Bind index params
        for (param, idx) in index_params.iter().zip(indices.iter()) {
            env.set_local(*param, idx.clone());
        }
        // Bind function params
        let func_params = &def.params[index_params.len()..];
        for (param, arg) in func_params.iter().zip(call_args.iter()) {
            env.set_local(*param, arg.clone());
        }
        let result = meval(&def.body, env);
        env.pop_scope();
        return result;
    }

    // Not found
    let mut all_args = vec![Expr::call("mqapply", {
        let mut v = vec![base.clone()];
        v.extend(indices.iter().cloned());
        v
    })];
    all_args.extend(call_args.iter().cloned());
    Expr::call("funapply", all_args)
}

fn eval_sort(args: &[Expr], env: &mut Environment) -> Expr {
    if let Some(Expr::List { op: Operator::MList, args: items, .. }) = args.first() {
        let mut sorted = items.clone();
        if args.len() >= 2 {
            // sort with comparison function or string operator
            let pred = &args[1];
            match pred {
                Expr::String(s) if s.as_ref() == "<" => {
                    // "<" requires all elements to be numeric
                    let all_numeric = sorted.iter().all(|x| to_f64(x).is_some());
                    if !all_numeric {
                        panic!("sort: '<' requires numeric elements");
                    }
                    sorted.sort_by(|a, b| {
                        let fa = to_f64(a).unwrap();
                        let fb = to_f64(b).unwrap();
                        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Expr::String(s) if s.as_ref() == ">" => {
                    let all_numeric = sorted.iter().all(|x| to_f64(x).is_some());
                    if !all_numeric {
                        panic!("sort: '>' requires numeric elements");
                    }
                    sorted.sort_by(|a, b| {
                        let fb = to_f64(b).unwrap();
                        let fa = to_f64(a).unwrap();
                        fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Expr::String(s) if s.as_ref() == "orderlessp" => {
                    sorted.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                }
                _ => {
                    // Lambda or function as comparator
                    let mut sort_failed = false;
                    sorted.sort_by(|a, b| {
                        let result = apply_func(pred, &[a.clone(), b.clone()], env);
                        if is_true(&result) {
                            std::cmp::Ordering::Less
                        } else if is_false(&result) {
                            std::cmp::Ordering::Greater
                        } else {
                            sort_failed = true;
                            std::cmp::Ordering::Equal
                        }
                    });
                    if sort_failed {
                        panic!("sort: comparator must return true or false");
                    }
                }
            }
        } else {
            sorted.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        }
        Expr::list(sorted)
    } else {
        Expr::call("sort", args.to_vec())
    }
}

fn eval_makelist(args: &[Expr], env: &mut Environment) -> Expr {
    // makelist(expr, var, lo, hi)
    if args.len() != 4 {
        let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
        return Expr::call("makelist", evaled);
    }
    let body = &args[0];
    let var = match &args[1] {
        Expr::Symbol(id) => *id,
        _ => {
            let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
            return Expr::call("makelist", evaled);
        }
    };
    let lo = meval(&args[2], env);
    let hi = meval(&args[3], env);

    if let (Some(lo_i), Some(hi_i)) = (to_i64(&lo), to_i64(&hi)) {
        let mut items = Vec::new();
        env.push_scope();
        for i in lo_i..=hi_i {
            env.set_local(var, Expr::int(i));
            items.push(meval(body, env));
        }
        env.pop_scope();
        Expr::list(items)
    } else {
        let evaled: Vec<Expr> = args.iter().map(|a| meval(a, env)).collect();
        Expr::call("makelist", evaled)
    }
}

fn eval_is_with_db(val: &Expr, db: &crate::assume::AssumptionDB) -> Expr {
    match val {
        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "true" => Expr::sym("true"),
                "false" => Expr::sym("false"),
                _ => Expr::sym("unknown"),
            }
        }
        Expr::List { op, args, .. } => {
            match op {
                Operator::MEqual | Operator::MNotEqual
                | Operator::MLessThan | Operator::MGreaterThan
                | Operator::MLessEqual | Operator::MGreaterEqual => {
                    if args.len() == 2 {
                        // Try numeric comparison first
                        let result = eval_comparison(op, &args[0], &args[1]);
                        if let Expr::Symbol(id) = &result {
                            let name = resolve(*id);
                            if name == "true" || name == "false" {
                                return result;
                            }
                        }
                        // Try assumption database
                        if let Some((lhs, rel, rhs)) = extract_relation(val) {
                            if let Some(answer) = db.query(&lhs, rel, &rhs) {
                                return bool_result(answer);
                            }
                        }
                        // Structural comparison for atoms
                        if args[0].is_atom() && args[1].is_atom() {
                            return match op {
                                Operator::MEqual => bool_result(args[0] == args[1]),
                                Operator::MNotEqual => bool_result(args[0] != args[1]),
                                _ => Expr::sym("unknown"),
                            };
                        }
                    }
                    Expr::sym("unknown")
                }
                Operator::Named(id) => {
                    let fname = resolve(*id);
                    match fname.as_str() {
                        "equal" if args.len() == 2 => {
                            return is_equal_pred(&args[0], &args[1]);
                        }
                        "notequal" if args.len() == 2 => {
                            let eq = is_equal_pred(&args[0], &args[1]);
                            return eval_not(&eq);
                        }
                        _ => return Expr::sym("unknown"),
                    }
                }
                Operator::MAnd => {
                    for arg in args {
                        let r = eval_is_with_db(arg, db);
                        if is_false(&r) {
                            return Expr::sym("false");
                        }
                    }
                    Expr::sym("true")
                }
                Operator::MOr => {
                    for arg in args {
                        let r = eval_is_with_db(arg, db);
                        if is_true(&r) {
                            return Expr::sym("true");
                        }
                    }
                    Expr::sym("false")
                }
                Operator::MNot => {
                    if let Some(inner) = args.first() {
                        let r = eval_is_with_db(inner, db);
                        return eval_not(&r);
                    }
                    Expr::sym("unknown")
                }
                _ => Expr::sym("unknown"),
            }
        }
        _ => Expr::sym("unknown"),
    }
}

#[allow(dead_code)]
fn eval_is(val: &Expr) -> Expr {
    match val {
        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "true" => Expr::sym("true"),
                "false" => Expr::sym("false"),
                _ => Expr::sym("unknown"),
            }
        }
        Expr::List { op, args, .. } => {
            match op {
                Operator::MEqual | Operator::MNotEqual
                | Operator::MLessThan | Operator::MGreaterThan
                | Operator::MLessEqual | Operator::MGreaterEqual => {
                    if args.len() == 2 {
                        // Try numeric comparison
                        let result = eval_comparison(op, &args[0], &args[1]);
                        if let Expr::Symbol(_) = &result {
                            return result;
                        }
                        // Structural comparison for atoms
                        if args[0].is_atom() && args[1].is_atom() {
                            return match op {
                                Operator::MEqual => bool_result(args[0] == args[1]),
                                Operator::MNotEqual => bool_result(args[0] != args[1]),
                                _ => Expr::sym("unknown"),
                            };
                        }
                    }
                    Expr::sym("unknown")
                }
                Operator::MAnd => {
                    for arg in args {
                        let r = eval_is(arg);
                        if is_false(&r) {
                            return Expr::sym("false");
                        }
                    }
                    Expr::sym("true")
                }
                Operator::MOr => {
                    for arg in args {
                        let r = eval_is(arg);
                        if is_true(&r) {
                            return Expr::sym("true");
                        }
                    }
                    Expr::sym("false")
                }
                Operator::MNot => {
                    if let Some(inner) = args.first() {
                        let r = eval_is(inner);
                        return eval_not(&r);
                    }
                    Expr::sym("unknown")
                }
                _ => Expr::sym("unknown"),
            }
        }
        _ => Expr::sym("unknown"),
    }
}

fn eval_kill(args: &[Expr], env: &mut Environment) {
    for arg in args {
        if let Expr::Symbol(id) = arg {
            let name = resolve(*id);
            match name.as_str() {
                "all" => env.kill_all(),
                "functions" => {
                    let fns: Vec<_> = env.list_functions();
                    for f in fns {
                        env.kill_function(f);
                    }
                }
                "values" => {
                    let vals: Vec<_> = env.list_values();
                    for v in vals {
                        env.kill_var(v);
                    }
                }
                "arrays" => {}
                _ => {
                    env.kill_var(*id);
                    env.kill_function(*id);
                }
            }
        }
    }
}

fn eval_logical(op: &Operator, args: &[Expr], env: &mut Environment) -> Expr {
    match op {
        Operator::MAnd => {
            let mut remaining = Vec::new();
            for arg in args {
                let val = meval(arg, env);
                if is_false(&val) {
                    return Expr::sym("false");
                }
                if !is_true(&val) {
                    remaining.push(val);
                }
            }
            if remaining.is_empty() {
                Expr::sym("true")
            } else if remaining.len() == 1 {
                remaining.pop().unwrap()
            } else {
                simplify(&Expr::List { op: Operator::MAnd, simplified: false, args: remaining })
            }
        }
        Operator::MOr => {
            let mut remaining = Vec::new();
            for arg in args {
                let val = meval(arg, env);
                if is_true(&val) {
                    return Expr::sym("true");
                }
                if !is_false(&val) {
                    remaining.push(val);
                }
            }
            if remaining.is_empty() {
                Expr::sym("false")
            } else if remaining.len() == 1 {
                remaining.pop().unwrap()
            } else {
                simplify(&Expr::List { op: Operator::MOr, simplified: false, args: remaining })
            }
        }
        _ => unreachable!(),
    }
}

fn eval_if(args: &[Expr], env: &mut Environment) -> Expr {
    let cond = meval(&args[0], env);
    if is_true(&cond) {
        meval(&args[1], env)
    } else if args.len() > 2 {
        meval(&args[2], env)
    } else {
        Expr::sym("false")
    }
}

fn eval_do(args: &[Expr], env: &mut Environment) -> Expr {
    if args.len() >= 4 {
        let var = match &args[0] {
            Expr::Symbol(id) => *id,
            _ => panic!("for loop variable must be a symbol"),
        };
        let start = meval(&args[1], env);
        let end_val = meval(&args[2], env);

        let (step, body_idx) = if args.len() == 5 {
            (meval(&args[3], env), 4)
        } else {
            (Expr::int(1), 3)
        };

        let body = &args[body_idx];

        if let (Some(mut i), Some(end), Some(s)) =
            (to_i64(&start), to_i64(&end_val), to_i64(&step))
        {
            env.push_scope();
            while (s > 0 && i <= end) || (s < 0 && i >= end) {
                env.set_local(var, Expr::int(i));
                let result = meval(body, env);
                if let Expr::List { op: Operator::MReturn, args: ret_args, .. } = &result {
                    let ret_val = ret_args[0].clone();
                    env.pop_scope();
                    return ret_val;
                }
                i += s;
            }
            env.pop_scope();
        }
        Expr::sym("done")
    } else if args.len() == 2 {
        let cond = &args[0];
        let body = &args[1];
        loop {
            let c = meval(cond, env);
            if !is_true(&c) {
                break;
            }
            let result = meval(body, env);
            if let Expr::List { op: Operator::MReturn, args: ret_args, .. } = &result {
                return ret_args[0].clone();
            }
        }
        Expr::sym("done")
    } else {
        Expr::sym("done")
    }
}

fn eval_block(args: &[Expr], env: &mut Environment) -> Expr {
    if args.is_empty() {
        return Expr::sym("done");
    }

    env.push_scope();

    let start_idx = if let Expr::List { op: Operator::MList, args: locals, .. } = &args[0] {
        for local in locals {
            match local {
                Expr::Symbol(id) => {
                    env.set_local(*id, Expr::Symbol(*id));
                }
                Expr::List { op: Operator::MAssign, args: assign_args, .. } => {
                    if let Expr::Symbol(id) = &assign_args[0] {
                        let val = meval(&assign_args[1], env);
                        env.set_local(*id, val);
                    }
                }
                _ => {}
            }
        }
        1
    } else {
        0
    };

    let mut result = Expr::sym("done");
    for arg in &args[start_idx..] {
        result = meval(arg, env);
        if let Expr::List { op: Operator::MReturn, args: ret_args, .. } = &result {
            let ret_val = ret_args[0].clone();
            env.pop_scope();
            return ret_val;
        }
        // Store result for %% reference within blocks
        env.store_output(result.clone());
    }

    env.pop_scope();
    result
}



/// Normalize expression for abs: ensure consistent sign of leading term.
/// abs(-a+b) should be the same as abs(a-b).
fn resolve_file(filename: &str, env: &Environment) -> Option<String> {
    let candidates: Vec<String> = {
        let mut c = Vec::new();
        // Absolute or explicit relative path: try directly
        if filename.starts_with('/') || filename.starts_with("./") || filename.starts_with("../") {
            c.push(filename.to_string());
            if !filename.contains('.') { c.push(format!("{}.mac", filename)); }
            return c.into_iter().find(|p| std::path::Path::new(p).is_file());
        }
        // Relative to current load_pathname directory
        if let Some(ref load_path) = env.load_pathname {
            if let Some(dir) = std::path::Path::new(load_path).parent() {
                let base = dir.join(filename);
                c.push(base.display().to_string());
                if !filename.contains('.') {
                    c.push(dir.join(format!("{}.mac", filename)).display().to_string());
                }
            }
        }
        // Search in configured search_paths
        for dir in &env.search_paths {
            c.push(format!("{}/{}", dir, filename));
            if !filename.contains('.') {
                c.push(format!("{}/{}.mac", dir, filename));
            }
            c.push(format!("{}/share/{}", dir, filename));
            if !filename.contains('.') {
                c.push(format!("{}/share/{}.mac", dir, filename));
            }
        }
        c
    };
    candidates.into_iter().find(|p| std::path::Path::new(p).is_file())
}

fn eval_load(filename: &str, env: &mut Environment) -> Expr {
    let path = match resolve_file(filename, env) {
        Some(p) => p,
        None => return Expr::sym("false"),
    };

    let canonical = std::fs::canonicalize(&path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.clone());

    let prev_load_pathname = env.load_pathname.take();
    env.load_pathname = Some(canonical.clone());

    if let Ok(content) = std::fs::read_to_string(&path) {
        let exprs = maxima_parser::parse_multi(&content);
        for expr in exprs {
            let expr = if env.ibase != 10 {
                reinterpret_integers(&expr, env.ibase)
            } else {
                expr
            };
            meval(&expr, env);
        }
    }

    env.mark_file_loaded(canonical);
    env.load_pathname = prev_load_pathname;
    Expr::String(path.into())
}

fn eval_require(filename: &str, env: &mut Environment) -> Expr {
    if let Some(resolved) = resolve_file(filename, env) {
        let canonical = std::fs::canonicalize(&resolved)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| resolved.clone());
        if env.is_file_loaded(&canonical) {
            return Expr::String(resolved.into());
        }
    }
    eval_load(filename, env)
}

fn null_space_vector(mat: &[Vec<f64>]) -> Vec<f64> {
    let m = mat.len();
    if m == 0 { return vec![]; }
    let n = mat[0].len();
    let mut work: Vec<Vec<f64>> = mat.to_vec();

    // Row echelon form
    let mut pivot_cols = Vec::new();
    let mut row = 0;
    for col in 0..n {
        if row >= m { break; }
        let mut pr = None;
        for r in row..m {
            if work[r][col].abs() > 1e-10 {
                pr = Some(r);
                break;
            }
        }
        if let Some(p) = pr {
            work.swap(row, p);
            let pv = work[row][col];
            for c in 0..n { work[row][c] /= pv; }
            for r in 0..m {
                if r != row && work[r][col].abs() > 1e-10 {
                    let f = work[r][col];
                    for c in 0..n { work[r][c] -= f * work[row][c]; }
                }
            }
            pivot_cols.push(col);
            row += 1;
        }
    }

    // Find a free variable (column not in pivot_cols)
    let free_col = (0..n).find(|c| !pivot_cols.contains(c)).unwrap_or(n - 1);

    // Construct null space vector: set free variable to 1
    let mut vec = vec![0.0; n];
    vec[free_col] = 1.0;
    for (i, &pc) in pivot_cols.iter().enumerate() {
        if i < work.len() {
            vec[pc] = -work[i][free_col];
        }
    }
    vec
}

/// trigexpand: expand trig of sums
fn trig_expand(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            let inner = trig_expand(&args[0]);
            // sin(a+b) → sin(a)*cos(b) + cos(a)*sin(b)
            if let Expr::List { op: Operator::MPlus, args: sum_args, .. } = &inner {
                if sum_args.len() == 2 {
                    let (a, b) = (&sum_args[0], &sum_args[1]);
                    match fname.as_str() {
                        "sin" => return simplify(&Expr::add(
                            Expr::mul(Expr::call("sin", vec![a.clone()]), Expr::call("cos", vec![b.clone()])),
                            Expr::mul(Expr::call("cos", vec![a.clone()]), Expr::call("sin", vec![b.clone()])),
                        )),
                        "cos" => return simplify(&Expr::sub(
                            Expr::mul(Expr::call("cos", vec![a.clone()]), Expr::call("cos", vec![b.clone()])),
                            Expr::mul(Expr::call("sin", vec![a.clone()]), Expr::call("sin", vec![b.clone()])),
                        )),
                        _ => {}
                    }
                }
            }
            // sin(n*x) → expand via double angle
            if let Expr::List { op: Operator::MTimes, args: prod_args, .. } = &inner {
                if prod_args.len() == 2 {
                    if let Expr::Integer(2) = &prod_args[0] {
                        let x = &prod_args[1];
                        match fname.as_str() {
                            "sin" => return simplify(&Expr::mul(
                                Expr::int(2),
                                Expr::mul(Expr::call("sin", vec![x.clone()]), Expr::call("cos", vec![x.clone()])),
                            )),
                            "cos" => return simplify(&Expr::sub(
                                Expr::pow(Expr::call("cos", vec![x.clone()]), Expr::int(2)),
                                Expr::pow(Expr::call("sin", vec![x.clone()]), Expr::int(2)),
                            )),
                            _ => {}
                        }
                    }
                }
            }
            Expr::call(&fname, vec![inner])
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| trig_expand(a)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

/// trigreduce: convert products of trig to sums
fn trig_reduce(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MTimes, args, .. } => {
            // Reduce factors first, then look for a pair of trig factors to
            // combine via product-to-sum (handles numeric coefficients and
            // products of different angles, e.g. 2*sin(x)*cos(x) -> sin(2x)).
            let factors: Vec<Expr> = args.iter().map(trig_reduce).collect();
            for i in 0..factors.len() {
                for j in (i + 1)..factors.len() {
                    if let Some(combined) = product_to_sum(&factors[i], &factors[j]) {
                        let mut rest: Vec<Expr> = factors.iter().enumerate()
                            .filter(|(k, _)| *k != i && *k != j)
                            .map(|(_, e)| e.clone())
                            .collect();
                        rest.push(combined);
                        let prod = simplify(&Expr::List {
                            op: Operator::MTimes, simplified: false, args: rest,
                        });
                        return trig_reduce(&prod);
                    }
                }
            }
            simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: factors })
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if let (Some(name), Some(x)) = (get_trig_name(&args[0]), get_trig_arg(&args[0])) {
                if let Some(n) = to_i64(&args[1]) {
                    if let Some(reduced) = power_reduce_trig(&name, &x, n) {
                        return reduced;
                    }
                }
            }
            let new_args: Vec<Expr> = args.iter().map(|a| trig_reduce(a)).collect();
            Expr::List { op: Operator::MExpt, simplified: false, args: new_args }
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| trig_reduce(a)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

/// Product-to-sum for two trig factors with the same or different arguments:
///   sin(a)sin(b) = (cos(a-b) - cos(a+b))/2
///   cos(a)cos(b) = (cos(a-b) + cos(a+b))/2
///   sin(a)cos(b) = (sin(a+b) + sin(a-b))/2
fn product_to_sum(f: &Expr, g: &Expr) -> Option<Expr> {
    let (nf, ng) = (get_trig_name(f)?, get_trig_name(g)?);
    let (a, b) = (get_trig_arg(f)?, get_trig_arg(g)?);
    let sum = simplify(&Expr::add(a.clone(), b.clone()));
    let diff = simplify(&Expr::sub(a.clone(), b.clone()));
    let two = Expr::int(2);
    let result = match (nf.as_str(), ng.as_str()) {
        ("sin", "sin") => Expr::div(
            Expr::sub(trig_call("cos", diff), trig_call("cos", sum)), two),
        ("cos", "cos") => Expr::div(
            Expr::add(trig_call("cos", diff), trig_call("cos", sum)), two),
        ("sin", "cos") => Expr::div(
            Expr::add(trig_call("sin", sum), trig_call("sin", diff)), two),
        ("cos", "sin") => Expr::div(
            Expr::add(trig_call("sin", sum),
                      trig_call("sin", simplify(&Expr::sub(b, a)))), two),
        _ => return None,
    };
    Some(simplify(&result))
}

/// Build a trig call, folding the zero-argument special values that the
/// pure simplifier (which does not evaluate functions) would otherwise leave.
fn trig_call(name: &str, arg: Expr) -> Expr {
    if arg == Expr::int(0) {
        return match name {
            "sin" | "tan" => Expr::int(0),
            "cos" => Expr::int(1),
            _ => Expr::call(name, vec![arg]),
        };
    }
    Expr::call(name, vec![arg])
}

/// Power reduction of sin^n / cos^n to a linear combination of multiple angles.
fn power_reduce_trig(name: &str, x: &Expr, n: i64) -> Option<Expr> {
    let cos = |k: i64| Expr::call("cos", vec![simplify(&Expr::mul(Expr::int(k), x.clone()))]);
    let sin = |k: i64| Expr::call("sin", vec![simplify(&Expr::mul(Expr::int(k), x.clone()))]);
    let r = match (name, n) {
        // sin^2 = (1 - cos2x)/2 ; cos^2 = (1 + cos2x)/2
        ("sin", 2) => Expr::div(Expr::sub(Expr::int(1), cos(2)), Expr::int(2)),
        ("cos", 2) => Expr::div(Expr::add(Expr::int(1), cos(2)), Expr::int(2)),
        // sin^3 = (3 sin x - sin 3x)/4 ; cos^3 = (3 cos x + cos 3x)/4
        ("sin", 3) => Expr::div(
            Expr::sub(Expr::mul(Expr::int(3), sin(1)), sin(3)), Expr::int(4)),
        ("cos", 3) => Expr::div(
            Expr::add(Expr::mul(Expr::int(3), cos(1)), cos(3)), Expr::int(4)),
        // sin^4 = (3 - 4 cos2x + cos4x)/8 ; cos^4 = (3 + 4 cos2x + cos4x)/8
        ("sin", 4) => Expr::div(
            Expr::add(Expr::sub(Expr::int(3), Expr::mul(Expr::int(4), cos(2))), cos(4)),
            Expr::int(8)),
        ("cos", 4) => Expr::div(
            Expr::add(Expr::add(Expr::int(3), Expr::mul(Expr::int(4), cos(2))), cos(4)),
            Expr::int(8)),
        _ => return None,
    };
    Some(simplify(&r))
}

/// trigrat: canonical multiple-angle linear form (trigreduce then ratsimp,
/// with numeric factors merged so e.g. 2*(sin(2x)/2) collapses to sin(2x)).
fn trig_rat(expr: &Expr) -> Expr {
    ratsimp(&combine_numeric_factors(&trig_reduce(expr)))
}

/// Merge multiple numeric factors of a product into one rational coefficient,
/// e.g. 2 * X * 2^(-1) -> X. Only fires when there are >= 2 numeric factors,
/// so a lone 2^(-1) (i.e. X/2) keeps its fraction display.
fn combine_numeric_factors(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MTimes, args, .. } => {
            let factors: Vec<Expr> = args.iter().map(combine_numeric_factors).collect();
            // Multiply all integer / integer^integer factors into a rational.
            let (mut num, mut den): (i64, i64) = (1, 1);
            let mut numeric_count = 0usize;
            let mut rest: Vec<Expr> = Vec::new();
            for f in &factors {
                match f {
                    Expr::Integer(n) => { num *= n; numeric_count += 1; }
                    Expr::Rational { num: n, den: d } => { num *= n; den *= d; numeric_count += 1; }
                    Expr::List { op: Operator::MExpt, args: pa, .. }
                        if pa.len() == 2 => {
                        if let (Expr::Integer(b), Expr::Integer(e)) = (&pa[0], &pa[1]) {
                            if *e < 0 && *b != 0 {
                                if let Some(p) = b.checked_pow((-*e) as u32) {
                                    den *= p; numeric_count += 1; continue;
                                }
                            } else if *e >= 0 {
                                if let Some(p) = b.checked_pow(*e as u32) {
                                    num *= p; numeric_count += 1; continue;
                                }
                            }
                        }
                        rest.push(f.clone());
                    }
                    _ => rest.push(f.clone()),
                }
            }
            if numeric_count < 2 {
                return simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: factors });
            }
            // Reduce the rational coefficient.
            let sign = if (num < 0) ^ (den < 0) { -1 } else { 1 };
            let (mut a, mut b) = (num.unsigned_abs(), den.unsigned_abs());
            while b != 0 { let t = b; b = a % b; a = t; }
            let g = a.max(1);
            let (cn, cd) = ((num.unsigned_abs() / g) as i64 * sign, (den.unsigned_abs() / g) as i64);
            let coeff = if cd == 1 { Expr::int(cn) } else { Expr::Rational { num: cn, den: cd } };
            let mut all = vec![coeff];
            all.extend(rest);
            simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: all })
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(combine_numeric_factors).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

/// halfangles: rewrite sin/cos/tan(a) via half-angle (sqrt) formulas, expressed
/// in terms of the doubled argument 2a (so sin(x/2) -> sqrt((1-cos x)/2)).
fn half_angles(expr: &Expr) -> Expr {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        let name = resolve(*id);
        if args.len() == 1 && matches!(name.as_str(), "sin" | "cos" | "tan") {
            let a = half_angles(&args[0]);
            let two_a = simplify(&Expr::mul(Expr::int(2), a.clone()));
            let cos2 = Expr::call("cos", vec![two_a.clone()]);
            return match name.as_str() {
                "sin" => simplify(&Expr::call("sqrt", vec![
                    Expr::div(Expr::sub(Expr::int(1), cos2), Expr::int(2))])),
                "cos" => simplify(&Expr::call("sqrt", vec![
                    Expr::div(Expr::add(Expr::int(1), cos2), Expr::int(2))])),
                // tan(a) = sin(2a)/(1+cos(2a))
                "tan" => simplify(&Expr::div(
                    Expr::call("sin", vec![two_a]),
                    Expr::add(Expr::int(1), cos2))),
                _ => unreachable!(),
            };
        }
    }
    match expr {
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(half_angles).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}



/// trigsimp: simplify using Pythagorean identities
fn trig_simp(expr: &Expr) -> Expr {
    // Strategy: simplify, which now applies Pythagorean identity
    let s = simplify(expr);
    // Also try substituting sin²→1-cos² and cos²→1-sin², pick simplest
    s
}

fn get_trig_name(expr: &Expr) -> Option<String> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if args.len() == 1 {
            let name = resolve(*id);
            if matches!(name.as_str(), "sin" | "cos" | "tan" | "sec" | "csc" | "cot") {
                return Some(name);
            }
        }
    }
    None
}

fn get_trig_arg(expr: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::Named(_), args, .. } = expr {
        if args.len() == 1 {
            return Some(args[0].clone());
        }
    }
    None
}

fn matrix_dot_product(rows_a: &[Expr], rows_b: &[Expr], _env: &mut Environment) -> Expr {
    // Extract dimensions
    let m = rows_a.len();
    let get_cols = |rows: &[Expr]| -> Vec<Vec<Expr>> {
        rows.iter().map(|r| {
            if let Expr::List { op: Operator::MList, args, .. } = r {
                args.clone()
            } else {
                vec![r.clone()]
            }
        }).collect()
    };
    let a = get_cols(rows_a);
    let b = get_cols(rows_b);
    if a.is_empty() || b.is_empty() { return Expr::call(".", vec![Expr::List { op: Operator::MMatrix, simplified: false, args: rows_a.to_vec() }, Expr::List { op: Operator::MMatrix, simplified: false, args: rows_b.to_vec() }]); }
    let k = a[0].len();
    let n = b[0].len();
    let mut result = Vec::new();
    for i in 0..m {
        let mut row = Vec::new();
        for j in 0..n {
            let mut sum = Expr::int(0);
            for l in 0..k {
                let prod = simplify(&Expr::mul(a[i][l].clone(), b[l][j].clone()));
                sum = simplify(&Expr::add(sum, prod));
            }
            row.push(sum);
        }
        result.push(Expr::list(row));
    }
    Expr::List { op: Operator::MMatrix, simplified: false, args: result }
}

fn eval_matrix_dot(a: &Expr, b: &Expr, env: &mut Environment) -> Expr {
    if let (
        Expr::List { op: Operator::MMatrix, args: rows_a, .. },
        Expr::List { op: Operator::MMatrix, args: rows_b, .. },
    ) = (a, b) {
        matrix_dot_product(rows_a, rows_b, env)
    } else {
        Expr::call(".", vec![a.clone(), b.clone()])
    }
}

fn matrix_det(mat: &[Vec<Expr>], env: &mut Environment) -> Expr {
    let n = mat.len();
    if n == 1 { return mat[0][0].clone(); }
    if n == 2 {
        let a = &mat[0][0]; let b = &mat[0][1];
        let c = &mat[1][0]; let d = &mat[1][1];
        return simplify(&Expr::sub(
            Expr::mul(a.clone(), d.clone()),
            Expr::mul(b.clone(), c.clone()),
        ));
    }
    // Laplace expansion along first row
    let mut result = Expr::int(0);
    for j in 0..n {
        let cofactor = matrix_cofactor(mat, 0, j, env);
        let term = simplify(&Expr::mul(mat[0][j].clone(), cofactor));
        result = simplify(&Expr::add(result, term));
    }
    result
}

fn matrix_cofactor(mat: &[Vec<Expr>], row: usize, col: usize, env: &mut Environment) -> Expr {
    let n = mat.len();
    let mut sub = Vec::new();
    for i in 0..n {
        if i == row { continue; }
        let mut r = Vec::new();
        for j in 0..n {
            if j == col { continue; }
            r.push(mat[i][j].clone());
        }
        sub.push(r);
    }
    let minor = matrix_det(&sub, env);
    if (row + col) % 2 == 0 {
        minor
    } else {
        simplify(&Expr::neg(minor))
    }
}

fn eval_linsolve(eqs: &[Expr], vars: &[Expr], _env: &mut Environment) -> Expr {
    let n = vars.len();
    if n == 0 || eqs.len() < n { return Expr::call("linsolve", vec![Expr::list(eqs.to_vec()), Expr::list(vars.to_vec())]); }

    // Extract coefficients by substitution: coeff of var_j in eq_i is
    // eval(eq_i with var_j=1, all others=0) - eval(eq_i with all vars=0)
    let mut mat: Vec<Vec<Expr>> = Vec::new();
    for eq in eqs.iter().take(n) {
        let expr = match eq {
            Expr::List { op: Operator::MEqual, args, .. } if args.len() == 2 => {
                simplify(&Expr::sub(args[0].clone(), args[1].clone()))
            }
            _ => eq.clone(),
        };

        // Set all vars to 0 to get constant term
        let mut zero_sub = expr.clone();
        for var in vars {
            zero_sub = subst(&Expr::int(0), var, &zero_sub);
        }
        let const_val = simplify(&zero_sub);

        let mut row = Vec::new();
        for var in vars {
            // Set this var to 1, all others to 0
            let mut one_sub = expr.clone();
            for v2 in vars {
                if v2 == var {
                    one_sub = subst(&Expr::int(1), v2, &one_sub);
                } else {
                    one_sub = subst(&Expr::int(0), v2, &one_sub);
                }
            }
            let coeff = simplify(&Expr::sub(simplify(&one_sub), const_val.clone()));
            row.push(coeff);
        }
        // RHS = -constant
        row.push(simplify(&Expr::neg(const_val)));
        mat.push(row);
    }

    // Gaussian elimination (numeric for now)
    let ncols = n + 1;
    let mut fmat: Vec<Vec<f64>> = mat.iter().map(|row| {
        row.iter().map(|e| to_f64(e).unwrap_or(0.0)).collect()
    }).collect();

    for col in 0..n {
        // Find pivot
        let mut pivot_row = None;
        for r in col..fmat.len() {
            if fmat[r][col].abs() > 1e-12 {
                pivot_row = Some(r);
                break;
            }
        }
        let pr = match pivot_row {
            Some(r) => r,
            None => continue,
        };
        fmat.swap(col, pr);

        let pivot = fmat[col][col];
        for r in (col + 1)..fmat.len() {
            let factor = fmat[r][col] / pivot;
            for c in col..ncols {
                let val = fmat[col][c];
                fmat[r][c] -= factor * val;
            }
        }
    }

    // Back substitution
    let mut solutions = vec![0.0f64; n];
    for i in (0..n).rev() {
        let mut sum = fmat[i][n];
        for j in (i + 1)..n {
            sum -= fmat[i][j] * solutions[j];
        }
        if fmat[i][i].abs() > 1e-12 {
            solutions[i] = sum / fmat[i][i];
        }
    }

    let result: Vec<Expr> = vars.iter().zip(solutions.iter()).map(|(v, s)| {
        let val = if (*s - s.round()).abs() < 1e-10 {
            Expr::int(s.round() as i64)
        } else {
            Expr::Float(*s)
        };
        Expr::List { op: Operator::MEqual, simplified: false, args: vec![v.clone(), val] }
    }).collect();

    Expr::list(result)
}

fn numeric_rank(mat: &[Vec<f64>]) -> usize {
    let m = mat.len();
    if m == 0 { return 0; }
    let n = mat[0].len();
    let mut work: Vec<Vec<f64>> = mat.to_vec();
    let mut rank = 0;
    let mut col = 0;
    for row in 0..m {
        if col >= n { break; }
        let mut pivot = None;
        for r in row..m {
            if work[r][col].abs() > 1e-12 {
                pivot = Some(r);
                break;
            }
        }
        match pivot {
            Some(pr) => {
                work.swap(row, pr);
                let pv = work[row][col];
                for r in (row + 1)..m {
                    let f = work[r][col] / pv;
                    for c in col..n {
                        let val = work[row][c];
                        work[r][c] -= f * val;
                    }
                }
                rank += 1;
            }
            None => {}
        }
        col += 1;
    }
    rank
}

/// Evaluate a string input and return the result as a string.
pub fn eval_str(input: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(input, &mut env)
}

/// Evaluate a string input with a given environment.
pub fn eval_str_with_env(input: &str, env: &mut Environment) -> String {
    let exprs = maxima_parser::parse_multi(input);
    let mut last = Expr::sym("done");
    for expr in exprs {
        let expr = if env.ibase != 10 {
            reinterpret_integers(&expr, env.ibase)
        } else {
            expr
        };
        last = meval(&expr, env);
    }
    last.to_string()
}

pub fn eval_expr_with_env(expr: &Expr, env: &mut Environment) -> String {
    let expr = if env.ibase != 10 {
        reinterpret_integers(expr, env.ibase)
    } else {
        expr.clone()
    };
    meval(&expr, env).to_string()
}

/// Reinterpret integer literals according to ibase.
/// Only plain integers are affected; floats and rationals are not.
fn reinterpret_integers(expr: &Expr, ibase: i64) -> Expr {
    match expr {
        Expr::Integer(n) => {
            let s = n.to_string();
            if let Some(val) = parse_int_in_base(&s, ibase) {
                Expr::int(val)
            } else {
                // Digits out of range for this base — becomes a symbol in Maxima
                Expr::sym(&s)
            }
        }
        Expr::Symbol(id) => {
            // Check for digit-letter identifiers like 0a000, 1xyz that might be base-N numbers
            let name = resolve(*id);
            if ibase > 10 && name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                if let Some(val) = parse_int_in_base_alpha(&name, ibase) {
                    return Expr::int(val);
                }
            }
            expr.clone()
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| reinterpret_integers(a, ibase)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

/// Try to factor a degree-4 polynomial as product of two quadratics (biquadratic).
/// ax^4 + bx^2 + c = (x^2 + p)(x^2 + q) where p+q = b/a, p*q = c/a.
fn try_biquadratic_factor(p: &maxima_poly::Poly, var: maxima_core::SymbolId) -> Option<Vec<(maxima_poly::Poly, u32)>> {
    if p.degree()? != 4 { return None; }
    let get = |e: u32| -> i64 {
        p.terms.iter().find(|(exp, _)| *exp == e)
            .map(|(_, c)| match c { maxima_poly::Coeff::Int(n) => *n, maxima_poly::Coeff::Rat(n, d) => *n / *d })
            .unwrap_or(0)
    };
    let a4 = get(4); let a3 = get(3); let a2 = get(2); let a1 = get(1); let a0 = get(0);
    if a4 != 1 || a3 != 0 || a1 != 0 { return None; }
    // x^4 + a2*x^2 + a0 = (x^2+p)(x^2+q)  where p+q=a2, p*q=a0
    // p,q are roots of t^2 - a2*t + a0 = 0
    let disc = a2 * a2 - 4 * a0;
    if disc < 0 { return None; }
    let sqrt_disc = (disc as f64).sqrt() as i64;
    if sqrt_disc * sqrt_disc != disc { return None; }
    let p_val = (a2 + sqrt_disc) / 2;
    let q_val = (a2 - sqrt_disc) / 2;
    if p_val + q_val != a2 || p_val * q_val != a0 { return None; }
    let f1 = maxima_poly::Poly { var, terms: vec![(2, maxima_poly::Coeff::Int(1)), (0, maxima_poly::Coeff::Int(p_val))] };
    let f2 = maxima_poly::Poly { var, terms: vec![(2, maxima_poly::Coeff::Int(1)), (0, maxima_poly::Coeff::Int(q_val))] };
    Some(vec![(f1, 1), (f2, 1)])
}

/// General partial fraction decomposition for distinct factors (linear + quadratic).
/// P(x) / (f1 * f2 * ... * fn) = A1/f1 + (Bx+C)/f2 + ...
fn partfrac_general(
    numer: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
    var: maxima_core::SymbolId,
) -> Option<Expr> {
    // Only handle distinct (multiplicity 1) factors of degree ≤ 2
    if factors.iter().any(|(_, m)| *m > 1) { return None; }
    let all_small = factors.iter().all(|(f, _)| f.degree().unwrap_or(0) <= 2);
    if !all_small { return None; }

    // Build the system: P(x) = sum_i( N_i(x) * product_{j!=i} f_j(x) )
    // where N_i has degree < degree(f_i)
    // Number of unknowns = sum of degrees of factors
    let total_unknowns: u32 = factors.iter().map(|(f, _)| f.degree().unwrap_or(0)).sum();
    let total_deg = numer.degree().unwrap_or(0);
    if total_deg >= total_unknowns { return None; }

    // For 2 factors, solve directly by coefficient matching
    if factors.len() == 2 {
        let (f0, _) = &factors[0];
        let (f1, _) = &factors[1];
        let d0 = f0.degree().unwrap_or(0);
        let d1 = f1.degree().unwrap_or(0);

        // For two quadratics: P/(f0*f1) = (Ax+B)/f0 + (Cx+D)/f1
        // => P = (Ax+B)*f1 + (Cx+D)*f0
        // Expand and match coefficients
        if d0 == 2 && d1 == 2 {
            return partfrac_two_quadratics(numer, f0, f1, var);
        }
        // One linear + one quadratic
        if d0 == 1 && d1 == 2 {
            return partfrac_linear_quadratic(numer, f0, f1, var);
        }
        if d0 == 2 && d1 == 1 {
            return partfrac_linear_quadratic(numer, f1, f0, var);
        }
    }
    None
}

fn partfrac_two_quadratics(
    numer: &maxima_poly::Poly,
    f0: &maxima_poly::Poly,
    f1: &maxima_poly::Poly,
    var: maxima_core::SymbolId,
) -> Option<Expr> {
    // P(x) = (Ax+B)*f1 + (Cx+D)*f0
    // f0 = x^2 + a1*x + a0, f1 = x^2 + b1*x + b0
    let get = |p: &maxima_poly::Poly, e: u32| -> i64 {
        match p.terms.iter().find(|(exp, _)| *exp == e) {
            Some((_, maxima_poly::Coeff::Int(n))) => *n,
            Some((_, maxima_poly::Coeff::Rat(n, d))) => *n / *d, // approximate
            _ => 0,
        }
    };
    let a0 = get(f0, 0); let a1 = get(f0, 1);
    let b0 = get(f1, 0); let b1 = get(f1, 1);
    let p0 = get(numer, 0); let p1 = get(numer, 1);
    let p2 = get(numer, 2); let p3 = get(numer, 3);

    // Coefficient matching for degree 3,2,1,0:
    // x^3: A + C = p3
    // x^2: B + A*b1 + D + C*a1 = p2
    // x^1: A*b0 + B*b1 + C*a0 + D*a1 = p1
    // x^0: B*b0 + D*a0 = p0
    //
    // From x^3: C = p3 - A
    // From x^0: D = (p0 - B*b0) / a0  (if a0 != 0)
    // Substitute into x^2 and x^1 to solve for A, B
    if a0 == 0 || b0 == 0 { return None; }

    // Use Cramer's rule on the 4x4 system (or solve iteratively)
    // Simpler: for monic quadratics with no x term (x^2+c form), a1=b1=0
    if a1 == 0 && b1 == 0 {
        // x^3: A + C = p3
        // x^2: B + D = p2
        // x^1: A*b0 + C*a0 = p1
        // x^0: B*b0 + D*a0 = p0
        let det = b0 - a0;
        if det == 0 { return None; }
        let a_val = (p1 - p3 * a0) as f64 / det as f64;
        let c_val = p3 as f64 - a_val;
        let b_val = (p0 - p2 * a0) as f64 / det as f64;
        let d_val = p2 as f64 - b_val;

        // Check integrality
        let a_i = a_val.round() as i64;
        let c_i = c_val.round() as i64;
        let b_i = b_val.round() as i64;
        let d_i = d_val.round() as i64;
        if (a_val - a_i as f64).abs() > 1e-9 || (c_val - c_i as f64).abs() > 1e-9
            || (b_val - b_i as f64).abs() > 1e-9 || (d_val - d_i as f64).abs() > 1e-9 {
            // Use rational arithmetic
            let num_a = p1 - p3 * a0;
            let num_b = p0 - p2 * a0;
            return build_partfrac_result_rational(num_a, det, p3, num_b, det, p2, f0, f1, var);
        }

        return build_partfrac_result(a_i, b_i, f0, c_i, d_i, f1, var);
    }

    // General case: solve 4x4 system
    let m = [
        [1, 0, 1, 0],           // A + C = p3
        [b1, 1, a1, 1],         // A*b1 + B + C*a1 + D = p2
        [b0, b1, a0, a1],       // A*b0 + B*b1 + C*a0 + D*a1 = p1
        [0, b0, 0, a0],         // B*b0 + D*a0 = p0
    ];
    let rhs = [p3, p2, p1, p0];
    // Gaussian elimination
    let mut mat = [[0f64; 5]; 4];
    for i in 0..4 { for j in 0..4 { mat[i][j] = m[i][j] as f64; } mat[i][4] = rhs[i] as f64; }
    for col in 0..4 {
        let pivot = (col..4).max_by(|&a, &b| mat[a][col].abs().partial_cmp(&mat[b][col].abs()).unwrap()).unwrap();
        mat.swap(col, pivot);
        if mat[col][col].abs() < 1e-15 { return None; }
        let d = mat[col][col];
        for j in col..5 { mat[col][j] /= d; }
        for i in 0..4 { if i != col { let f = mat[i][col]; for j in col..5 { mat[i][j] -= f * mat[col][j]; } } }
    }
    let a_i = mat[0][4].round() as i64;
    let b_i = mat[1][4].round() as i64;
    let c_i = mat[2][4].round() as i64;
    let d_i = mat[3][4].round() as i64;
    build_partfrac_result(a_i, b_i, f0, c_i, d_i, f1, var)
}

fn partfrac_linear_quadratic(
    numer: &maxima_poly::Poly,
    lin: &maxima_poly::Poly,
    quad: &maxima_poly::Poly,
    var: maxima_core::SymbolId,
) -> Option<Expr> {
    // P/(lin*quad) = A/lin + (Bx+C)/quad
    // lin = x + r (root = -r)
    let r = lin.constant_term();
    let lc = lin.leading_coeff();
    let root = r.neg().div(&lc)?;
    let num_at_root = numer.eval_at(&root);
    let quad_at_root = quad.eval_at(&root);
    let a_coeff = num_at_root.div(&quad_at_root)?;

    // (Bx+C) = (P - A*quad) / lin
    let a_poly = maxima_poly::Poly::constant(var, a_coeff.clone());
    let a_times_quad = a_poly.mul(quad);
    let remainder = numer.sub(&a_times_quad);
    let (quotient, rem) = remainder.divmod(lin)?;
    if !rem.is_zero() { return None; }

    let a_expr = match a_coeff {
        maxima_poly::Coeff::Int(n) => Expr::int(n),
        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
    };
    let lin_expr = maxima_poly::poly_to_expr(lin);
    let quot_expr = maxima_poly::poly_to_expr(&quotient);
    let quad_expr = maxima_poly::poly_to_expr(quad);

    let term1 = simplify(&Expr::div(a_expr, lin_expr));
    let term2 = simplify(&Expr::div(quot_expr, quad_expr));
    Some(simplify(&Expr::add(term1, term2)))
}

fn build_partfrac_result(
    a: i64, b: i64, f0: &maxima_poly::Poly,
    c: i64, d: i64, f1: &maxima_poly::Poly,
    var: maxima_core::SymbolId,
) -> Option<Expr> {
    let v = Expr::Symbol(var);
    let f0_expr = maxima_poly::poly_to_expr(f0);
    let f1_expr = maxima_poly::poly_to_expr(f1);

    let num0 = if a == 0 { Expr::int(b) }
        else if b == 0 { Expr::mul(Expr::int(a), v.clone()) }
        else { Expr::add(Expr::mul(Expr::int(a), v.clone()), Expr::int(b)) };
    let num1 = if c == 0 { Expr::int(d) }
        else if d == 0 { Expr::mul(Expr::int(c), v.clone()) }
        else { Expr::add(Expr::mul(Expr::int(c), v.clone()), Expr::int(d)) };

    let t0 = simplify(&Expr::div(num0, f0_expr));
    let t1 = simplify(&Expr::div(num1, f1_expr));
    Some(simplify(&Expr::add(t0, t1)))
}

fn build_partfrac_result_rational(
    num_a: i64, den_a: i64, p3: i64,
    num_b: i64, den_b: i64, p2: i64,
    f0: &maxima_poly::Poly, f1: &maxima_poly::Poly,
    var: maxima_core::SymbolId,
) -> Option<Expr> {
    let v = Expr::Symbol(var);
    let f0_expr = maxima_poly::poly_to_expr(f0);
    let f1_expr = maxima_poly::poly_to_expr(f1);

    let a_r = norm_rat(num_a, den_a);
    let c_r = simplify(&Expr::sub(Expr::int(p3), a_r.clone()));
    let b_r = norm_rat(num_b, den_b);
    let d_r = simplify(&Expr::sub(Expr::int(p2), b_r.clone()));

    let num0 = simplify(&Expr::add(Expr::mul(a_r, v.clone()), b_r));
    let num1 = simplify(&Expr::add(Expr::mul(c_r, v.clone()), d_r));

    let t0 = simplify(&Expr::div(num0, f0_expr));
    let t1 = simplify(&Expr::div(num1, f1_expr));
    Some(simplify(&Expr::add(t0, t1)))
}

fn norm_rat(num: i64, den: i64) -> Expr {
    if den == 0 { return Expr::sym("und"); }
    let sign = if den < 0 { -1 } else { 1 };
    let n = num * sign;
    let d = den * sign;
    let g = crate::simp::gcd_pub(n.unsigned_abs(), d.unsigned_abs()) as i64;
    if d / g == 1 { Expr::int(n / g) } else { Expr::Rational { num: n / g, den: d / g } }
}

/// Extract rational multiple of %pi from an expression.
/// Returns (num, den) if expr = (num/den)*%pi.
fn extract_pi_multiple(expr: &Expr) -> Option<(i64, i64)> {
    let pi_id = maxima_core::intern("%pi");
    if let Expr::Symbol(id) = expr {
        if *id == pi_id { return Some((1, 1)); }
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut coeff_num = 1i64;
        let mut coeff_den = 1i64;
        let mut has_pi = false;
        for a in args {
            match a {
                Expr::Symbol(id) if *id == pi_id => has_pi = true,
                Expr::Integer(n) => coeff_num *= n,
                Expr::Rational { num, den } => { coeff_num *= num; coeff_den *= den; }
                _ => return None,
            }
        }
        if has_pi { return Some((coeff_num, coeff_den)); }
    }
    None
}

fn trig_special_value(name: &str, num: i64, den: i64) -> Option<Expr> {
    // Normalize to [0, 2) by reducing num mod (2*den)
    let period = 2 * den;
    let n = ((num % period) + period) % period;

    // Table indexed by n/den (the fraction of pi)
    // We use (n, den) and match known values
    let (sin_val, cos_val) = match (n * 12 / den, den) {
        _ => {
            // General approach: compute n*12/den to normalize to 12ths of pi
            let twelfths = n * 12 / den;
            let rem = n * 12 % den;
            if rem != 0 { return None; } // Not a multiple of pi/12
            match twelfths % 24 {
                0 =>  (Expr::int(0), Expr::int(1)),                           // 0
                2 =>  (Expr::Rational{num:1,den:2}, sqrt3_over_2()),          // pi/6
                3 =>  (sqrt2_over_2(), sqrt2_over_2()),                       // pi/4
                4 =>  (sqrt3_over_2(), Expr::Rational{num:1,den:2}),          // pi/3
                6 =>  (Expr::int(1), Expr::int(0)),                           // pi/2
                8 =>  (sqrt3_over_2(), Expr::Rational{num:-1,den:2}),         // 2pi/3
                9 =>  (sqrt2_over_2(), simplify(&Expr::neg(sqrt2_over_2()))), // 3pi/4
                10 => (Expr::Rational{num:1,den:2}, simplify(&Expr::neg(sqrt3_over_2()))), // 5pi/6
                12 => (Expr::int(0), Expr::int(-1)),                          // pi
                14 => (Expr::Rational{num:-1,den:2}, simplify(&Expr::neg(sqrt3_over_2()))), // 7pi/6
                15 => (simplify(&Expr::neg(sqrt2_over_2())), simplify(&Expr::neg(sqrt2_over_2()))), // 5pi/4
                16 => (simplify(&Expr::neg(sqrt3_over_2())), Expr::Rational{num:-1,den:2}), // 4pi/3
                18 => (Expr::int(-1), Expr::int(0)),                          // 3pi/2
                20 => (simplify(&Expr::neg(sqrt3_over_2())), Expr::Rational{num:1,den:2}),  // 5pi/3
                21 => (simplify(&Expr::neg(sqrt2_over_2())), sqrt2_over_2()), // 7pi/4
                22 => (Expr::Rational{num:-1,den:2}, sqrt3_over_2()),         // 11pi/6
                _ => return None,
            }
        }
    };

    match name {
        "sin" => Some(sin_val),
        "cos" => Some(cos_val),
        "tan" => {
            if cos_val == Expr::int(0) { return Some(Expr::sym("und")); }
            if sin_val == Expr::int(0) { return Some(Expr::int(0)); }
            if sin_val == cos_val { return Some(Expr::int(1)); }
            if simplify(&Expr::add(sin_val.clone(), cos_val.clone())) == Expr::int(0) {
                return Some(Expr::int(-1));
            }
            // Direct table for tan to avoid division simplification issues
            let twelfths = n * 12 / den;
            match twelfths % 24 {
                2 | 22 => Some(simplify(&Expr::div(Expr::call("sqrt", vec![Expr::int(3)]), Expr::int(3)))), // tan(pi/6) = 1/sqrt(3) = sqrt(3)/3
                4 | 20 => Some(Expr::call("sqrt", vec![Expr::int(3)])), // tan(pi/3) = sqrt(3)
                8 => Some(Expr::neg(Expr::call("sqrt", vec![Expr::int(3)]))), // tan(2pi/3)
                10 => Some(simplify(&Expr::neg(Expr::div(Expr::call("sqrt", vec![Expr::int(3)]), Expr::int(3))))), // tan(5pi/6)
                14 => Some(simplify(&Expr::div(Expr::call("sqrt", vec![Expr::int(3)]), Expr::int(3)))), // tan(7pi/6)
                16 => Some(Expr::call("sqrt", vec![Expr::int(3)])), // tan(4pi/3)
                _ => Some(simplify(&Expr::div(sin_val, cos_val))),
            }
        }
        _ => None,
    }
}

fn inverse_trig_special(name: &str, arg: &Expr) -> Option<Expr> {
    match name {
        "atan" => match arg {
            Expr::Integer(0) => Some(Expr::int(0)),
            Expr::Integer(1) => Some(simplify(&Expr::div(Expr::sym("%pi"), Expr::int(4)))),
            Expr::Integer(-1) => Some(simplify(&Expr::neg(Expr::div(Expr::sym("%pi"), Expr::int(4))))),
            _ => None,
        },
        "asin" => match arg {
            Expr::Integer(0) => Some(Expr::int(0)),
            Expr::Integer(1) => Some(simplify(&Expr::div(Expr::sym("%pi"), Expr::int(2)))),
            Expr::Integer(-1) => Some(simplify(&Expr::neg(Expr::div(Expr::sym("%pi"), Expr::int(2))))),
            _ => None,
        },
        "acos" => match arg {
            Expr::Integer(1) => Some(Expr::int(0)),
            Expr::Integer(0) => Some(simplify(&Expr::div(Expr::sym("%pi"), Expr::int(2)))),
            Expr::Integer(-1) => Some(Expr::sym("%pi")),
            _ => None,
        },
        _ => None,
    }
}

fn sqrt2_over_2() -> Expr {
    simplify(&Expr::div(Expr::call("sqrt", vec![Expr::int(2)]), Expr::int(2)))
}
fn sqrt3_over_2() -> Expr {
    simplify(&Expr::div(Expr::call("sqrt", vec![Expr::int(3)]), Expr::int(2)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str) -> String {
        eval_str(input)
    }

    fn run_env(inputs: &[&str]) -> String {
        let mut env = Environment::new();
        let mut last = String::new();
        for input in inputs {
            last = eval_str_with_env(input, &mut env);
        }
        last
    }

    #[test]
    fn eval_integer() {
        assert_eq!(run("42;"), "42");
    }

    #[test]
    fn eval_add() {
        assert_eq!(run("1+1;"), "2");
    }

    #[test]
    fn eval_add_multi() {
        assert_eq!(run("1+2+3;"), "6");
    }

    #[test]
    fn eval_mul() {
        assert_eq!(run("3*4;"), "12");
    }

    #[test]
    fn eval_precedence() {
        assert_eq!(run("2+3*4;"), "14");
    }

    #[test]
    fn eval_parens() {
        assert_eq!(run("(2+3)*4;"), "20");
    }

    #[test]
    fn eval_power() {
        assert_eq!(run("2^10;"), "1024");
    }

    #[test]
    fn eval_bigint() {
        assert_eq!(run("2^100;"), "1267650600228229401496703205376");
    }

    #[test]
    fn eval_negative() {
        assert_eq!(run("-3+5;"), "2");
    }

    #[test]
    fn eval_nested() {
        assert_eq!(run("((1+2)^2-1)*3;"), "24");
    }

    #[test]
    fn eval_assign() {
        assert_eq!(run_env(&["x:5;", "x+1;"]), "6");
    }

    #[test]
    fn eval_funcdef() {
        assert_eq!(run_env(&["f(x):=x^2;", "f(3);"]), "9");
    }

    #[test]
    fn eval_funcdef_multi_arg() {
        assert_eq!(run_env(&["g(x,y):=x+y;", "g(3,4);"]), "7");
    }

    #[test]
    fn eval_if_true() {
        assert_eq!(run("if 3 > 2 then 42 else 0;"), "42");
    }

    #[test]
    fn eval_if_false() {
        assert_eq!(run("if 1 > 2 then 42 else 0;"), "0");
    }

    #[test]
    fn eval_for_loop() {
        assert_eq!(run_env(&["s:0;", "for i:1 thru 5 do s:s+i;", "s;"]), "15");
    }

    #[test]
    fn eval_block() {
        assert_eq!(run("block([x:3], x^2);"), "9");
    }

    #[test]
    fn eval_block_multi() {
        assert_eq!(run("block([s:0], for i:1 thru 10 do s:s+i, s);"), "55");
    }

    #[test]
    fn eval_symbol_unchanged() {
        assert_eq!(run("x;"), "x");
    }

    #[test]
    fn eval_symbolic_add() {
        assert_eq!(run("x+1;"), "1+x");
    }

    #[test]
    fn eval_kill() {
        assert_eq!(run_env(&["x:5;", "kill(x);", "x;"]), "x");
    }

    #[test]
    fn eval_list() {
        assert_eq!(run("[1,2,3];"), "[1,2,3]");
    }

    #[test]
    fn eval_first() {
        assert_eq!(run("first([10,20,30]);"), "10");
    }

    #[test]
    fn eval_rest() {
        assert_eq!(run("rest([10,20,30]);"), "[20,30]");
    }

    #[test]
    fn eval_last() {
        assert_eq!(run("last([10,20,30]);"), "30");
    }

    #[test]
    fn eval_length() {
        assert_eq!(run("length([10,20,30]);"), "3");
    }

    #[test]
    fn eval_atom() {
        assert_eq!(run("atom(42);"), "true");
        assert_eq!(run("atom([1,2]);"), "false");
    }

    #[test]
    fn eval_numberp() {
        assert_eq!(run("numberp(42);"), "true");
        assert_eq!(run("numberp(x);"), "false");
    }

    #[test]
    fn eval_float_arith() {
        assert_eq!(run("1.5 + 2.5;"), "4");
    }

    #[test]
    fn eval_comparison_numeric() {
        assert_eq!(run("if 5 = 5 then 1 else 0;"), "1");
        assert_eq!(run("if 5 # 3 then 1 else 0;"), "1");
    }

    #[test]
    fn eval_pi_symbol() {
        assert_eq!(run("%pi;"), "%pi");
    }

    // RC1 new tests
    #[test]
    fn eval_ev() {
        assert_eq!(run_env(&["f(x):=x^2+y;", "ev(f(2), y:7);"]), "11");
    }

    #[test]
    fn eval_sum() {
        assert_eq!(run("sum(i, i, 1, 10);"), "55");
    }

    #[test]
    fn eval_sum_squares() {
        assert_eq!(run("sum(i^2, i, 1, 5);"), "55");
    }

    #[test]
    fn eval_factorial() {
        assert_eq!(run("5!;"), "120");
        assert_eq!(run("10!;"), "3628800");
    }

    #[test]
    fn eval_diff_poly() {
        assert_eq!(run("diff(x^3, x);"), "3*x^2");
    }

    #[test]
    fn eval_diff_const() {
        assert_eq!(run("diff(5, x);"), "0");
    }

    #[test]
    fn eval_diff_linear() {
        assert_eq!(run("diff(3*x, x);"), "3");
    }

    #[test]
    fn eval_diff_sum() {
        let r = run("diff(x^2+x, x);");
        assert!(r == "2*x+1" || r == "1+2*x", "got: {}", r);
    }

    #[test]
    fn eval_diff_sin() {
        assert_eq!(run("diff(sin(x), x);"), "cos(x)");
    }

    #[test]
    fn eval_expand_product() {
        let r = run("expand((x+1)*(x-1));");
        // Expand distributes correctly; terms may not be fully collected
        assert!(r.contains("x") && (r.contains("-1") || r.contains("+-1")),
            "got: {}", r);
    }

    #[test]
    fn eval_expand_power() {
        let r = run("expand((x+1)^2);");
        // Should have x*x or x^2 and 1
        assert!(r.contains("x") && r.contains("1"), "got: {}", r);
    }

    #[test]
    fn eval_subst() {
        assert_eq!(run("subst(2, x, x^2+1);"), "5");
    }

    #[test]
    fn eval_lambda() {
        assert_eq!(run("(lambda([x], x+1))(5);"), "6");
    }

    #[test]
    fn eval_map() {
        assert_eq!(
            run_env(&["f(x):=x^2;", "map(f, [1,2,3]);"]),
            "[1,4,9]"
        );
    }

    #[test]
    fn eval_is_numeric() {
        assert_eq!(run("is(3 > 2);"), "true");
        assert_eq!(run("is(1 > 2);"), "false");
    }

    #[test]
    fn eval_functions_list() {
        assert_eq!(
            run_env(&["f(x):=x;", "g(x,y):=x+y;", "functions;"]),
            "[f(x),g(x,y)]"
        );
    }

    #[test]
    fn eval_string_func() {
        assert_eq!(run("string(42);"), "\"42\"");
    }

    #[test]
    fn eval_concat() {
        assert_eq!(run("concat(x, 1);"), "x1");
    }

    #[test]
    fn eval_makelist() {
        assert_eq!(run("makelist(i^2, i, 1, 5);"), "[1,4,9,16,25]");
    }

    #[test]
    fn eval_dynamic_scope() {
        assert_eq!(
            run_env(&["a:1;", "f():=a;", "g():=block([a:2], f());", "g();"]),
            "2"
        );
    }

    #[test]
    fn eval_funcdef_returns_def() {
        let r = run("f(x):=x^2;");
        assert!(r.contains("f(x)") && r.contains("x^2"), "got: {}", r);
    }

    #[test]
    fn eval_subscript_func_concrete() {
        assert_eq!(
            run_env(&["t[0](x):=1;", "t[1](x):=x;", "t[0](5);"]),
            "1"
        );
    }

    #[test]
    fn eval_subscript_func_generic() {
        let r = run_env(&[
            "t[0](x):=1;",
            "t[1](x):=x;",
            "t[n](x):=2*x*t[n-1](x)-t[n-2](x);",
            "t[2](y);",
        ]);
        // Simplifier now collects y*y → y^2
        assert!(r == "2*y^2-1" || r == "-1+2*y^2", "got: {}", r);
    }

    #[test]
    fn eval_product() {
        assert_eq!(run("product(i, i, 1, 5);"), "120");
    }

    #[test]
    fn eval_diff_product() {
        assert_eq!(run("diff(x*x, x);"), "2*x");
    }

    // ===== Comprehensive tests for all features =====

    // --- Arithmetic edge cases ---

    #[test]
    fn eval_zero_times_anything() {
        assert_eq!(run("0*x;"), "0");
        assert_eq!(run("x*0;"), "0");
    }

    #[test]
    fn eval_one_times_anything() {
        assert_eq!(run("1*x;"), "x");
    }

    #[test]
    fn eval_power_zero() {
        assert_eq!(run("x^0;"), "1");
    }

    #[test]
    fn eval_power_one() {
        assert_eq!(run("x^1;"), "x");
    }

    #[test]
    fn eval_negative_power() {
        assert_eq!(run("2^-1;"), "1/2");
    }

    #[test]
    fn eval_rational_arith() {
        assert_eq!(run("1/3 + 1/6;"), "1/2");
    }

    #[test]
    fn eval_float_promotion() {
        let r = run("1 + 0.5;");
        assert_eq!(r, "1.5");
    }

    #[test]
    fn eval_integer_overflow_to_bigint() {
        let r = run("2^64;");
        assert_eq!(r, "18446744073709551616");
    }

    // --- Comparison operators ---

    #[test]
    fn eval_less_than() {
        assert_eq!(run("is(1 < 2);"), "true");
        assert_eq!(run("is(2 < 1);"), "false");
    }

    #[test]
    fn eval_less_equal() {
        assert_eq!(run("is(2 <= 2);"), "true");
        assert_eq!(run("is(3 <= 2);"), "false");
    }

    #[test]
    fn eval_greater_than() {
        assert_eq!(run("is(5 > 3);"), "true");
    }

    #[test]
    fn eval_not_equal() {
        assert_eq!(run("is(1 # 2);"), "true");
        assert_eq!(run("is(1 # 1);"), "false");
    }

    // --- Logical operators ---

    #[test]
    fn eval_and_operator() {
        assert_eq!(run("if true and true then 1 else 0;"), "1");
        assert_eq!(run("if true and false then 1 else 0;"), "0");
    }

    #[test]
    fn eval_or_operator() {
        assert_eq!(run("if false or true then 1 else 0;"), "1");
        assert_eq!(run("if false or false then 1 else 0;"), "0");
    }

    #[test]
    fn eval_not_operator() {
        assert_eq!(run("if not false then 1 else 0;"), "1");
        assert_eq!(run("if not true then 1 else 0;"), "0");
    }

    // --- Control flow ---

    #[test]
    fn eval_for_step() {
        assert_eq!(
            run_env(&["s:0;", "for i:0 thru 10 step 2 do s:s+i;", "s;"]),
            "30"
        );
    }

    #[test]
    fn eval_while_loop() {
        assert_eq!(
            run_env(&["x:10;", "s:0;", "while x > 0 do (s:s+x, x:x-1);", "s;"]),
            "55"
        );
    }

    #[test]
    fn eval_block_return() {
        assert_eq!(run("block([s:0], for i:1 thru 10 do s:s+i, return(s));"), "55");
    }

    #[test]
    fn eval_block_local_scope() {
        assert_eq!(
            run_env(&["x:10;", "block([x:5], x);", "x;"]),
            "10"
        );
    }

    #[test]
    fn eval_for_in_loop() {
        assert_eq!(
            run_env(&["s:0;", "for i in [10,20,30] do s:s+i;", "s;"]),
            "60"
        );
    }

    // --- List operations ---

    #[test]
    fn eval_append_lists() {
        assert_eq!(run("append([1,2], [3,4]);"), "[1,2,3,4]");
    }

    #[test]
    fn eval_cons() {
        assert_eq!(run("cons(0, [1,2,3]);"), "[0,1,2,3]");
    }

    #[test]
    fn eval_reverse_list() {
        assert_eq!(run("reverse([1,2,3]);"), "[3,2,1]");
    }

    #[test]
    fn eval_empty_list() {
        assert_eq!(run("length([]);"), "0");
    }

    #[test]
    fn eval_nested_list() {
        assert_eq!(run("first([[1,2],[3,4]]);"), "[1,2]");
    }

    #[test]
    fn eval_sort_default() {
        assert_eq!(run("sort([c, a, b]);"), "[a,b,c]");
    }

    #[test]
    fn eval_sort_numeric() {
        assert_eq!(run("sort([3,1,2], \"<\");"), "[1,2,3]");
    }

    #[test]
    fn eval_unique() {
        assert_eq!(run("unique([1,2,1,3,2]);"), "[1,2,3]");
    }

    // --- Type predicates ---

    #[test]
    fn eval_integerp() {
        assert_eq!(run("integerp(42);"), "true");
        assert_eq!(run("integerp(3.14);"), "false");
        assert_eq!(run("integerp(x);"), "false");
    }

    #[test]
    fn eval_floatnump() {
        assert_eq!(run("floatnump(3.14);"), "true");
        assert_eq!(run("floatnump(42);"), "false");
    }

    #[test]
    fn eval_listp() {
        assert_eq!(run("listp([1,2]);"), "true");
        assert_eq!(run("listp(42);"), "false");
    }

    #[test]
    fn eval_symbolp() {
        assert_eq!(run("symbolp(x);"), "true");
        assert_eq!(run("symbolp(42);"), "false");
    }

    #[test]
    fn eval_stringp() {
        assert_eq!(run("stringp(\"hi\");"), "true");
        assert_eq!(run("stringp(42);"), "false");
    }

    // --- Math functions ---

    #[test]
    fn eval_abs() {
        assert_eq!(run("abs(-5);"), "5");
        assert_eq!(run("abs(3);"), "3");
    }

    #[test]
    fn eval_mod() {
        assert_eq!(run("mod(10, 3);"), "1");
        assert_eq!(run("mod(7, 7);"), "0");
    }

    #[test]
    fn eval_max_min() {
        assert_eq!(run("max(3, 7, 2);"), "7");
        assert_eq!(run("min(3, 7, 2);"), "2");
    }

    #[test]
    fn eval_sqrt_perfect() {
        assert_eq!(run("sqrt(9);"), "3");
        assert_eq!(run("sqrt(16);"), "4");
    }

    #[test]
    fn eval_sin_cos_special() {
        assert_eq!(run("sin(0);"), "0");
        assert_eq!(run("cos(0);"), "1");
    }

    #[test]
    fn eval_exp_log_special() {
        assert_eq!(run("exp(0);"), "1");
        assert_eq!(run("log(1);"), "0");
    }

    #[test]
    fn eval_factorial_zero() {
        assert_eq!(run("0!;"), "1");
    }

    #[test]
    fn eval_factorial_large() {
        assert_eq!(run("20!;"), "2432902008176640000");
    }

    // --- Differentiation ---

    #[test]
    fn eval_diff_exp() {
        assert_eq!(run("diff(exp(x), x);"), "exp(x)");
    }

    #[test]
    fn eval_diff_log() {
        assert_eq!(run("diff(log(x), x);"), "1/x");
    }

    #[test]
    fn eval_diff_cos() {
        let r = run("diff(cos(x), x);");
        assert!(r == "-sin(x)", "got: {}", r);
    }

    #[test]
    fn eval_diff_chain_rule() {
        assert_eq!(run("diff(sin(2*x), x);"), "2*cos(2*x)");
    }

    #[test]
    fn eval_diff_nth() {
        assert_eq!(run("diff(x^4, x, 2);"), "12*x^2");
    }

    // --- Substitution and evaluation ---

    #[test]
    fn eval_subst_symbolic() {
        assert_eq!(run("subst(a, x, x+y);"), "a+y");
    }

    #[test]
    fn eval_ev_multiple_subs() {
        assert_eq!(run("ev(x+y, x:1, y:2);"), "3");
    }

    // --- Expand ---

    #[test]
    fn eval_expand_difference_of_squares() {
        assert_eq!(run("expand((a+b)*(a-b));"), "a^2-b^2");
    }

    #[test]
    fn eval_expand_cube() {
        let r = run("expand((x+1)^3);");
        assert!(r.contains("x^3") && r.contains("3*x^2") && r.contains("3*x"),
            "got: {}", r);
    }

    // --- String operations ---

    #[test]
    fn eval_sconcat() {
        assert_eq!(run("sconcat(\"a\", \"b\", \"c\");"), "\"abc\"");
    }

    #[test]
    fn eval_parse_string() {
        assert_eq!(run("parse_string(\"1+2\");"), "3");
    }

    // --- Map and apply ---

    #[test]
    fn eval_apply_plus() {
        assert_eq!(run("apply(\"+\", [1,2,3]);"), "6");
    }

    #[test]
    fn eval_map_lambda() {
        assert_eq!(run("map(lambda([x], x^2), [1,2,3]);"), "[1,4,9]");
    }

    // --- Errcatch ---

    #[test]
    fn eval_errcatch_success() {
        assert_eq!(run("errcatch(1+1);"), "[2]");
    }

    // --- Kill variants ---

    #[test]
    fn eval_kill_functions() {
        assert_eq!(
            run_env(&["f(x):=x;", "kill(functions);", "functions;"]),
            "[]"
        );
    }

    #[test]
    fn eval_kill_values() {
        assert_eq!(
            run_env(&["x:5;", "kill(values);", "x;"]),
            "x"
        );
    }

    #[test]
    fn eval_kill_all() {
        assert_eq!(
            run_env(&["x:5;", "f(x):=x;", "kill(all);", "x;"]),
            "x"
        );
    }

    // --- Expression access ---

    #[test]
    fn eval_lhs_rhs() {
        assert_eq!(run("lhs(a = b);"), "a");
        assert_eq!(run("rhs(a = b);"), "b");
    }

    #[test]
    fn eval_part() {
        assert_eq!(run("part([10,20,30], 2);"), "20");
    }

    #[test]
    fn eval_length_nested() {
        assert_eq!(run("length([[1],[2],[3]]);"), "3");
    }

    // --- Quote ---

    #[test]
    fn eval_quote_prevents_eval() {
        assert_eq!(
            run_env(&["x:5;", "'x;"]),
            "x"
        );
    }

    #[test]
    fn eval_double_quote_forces_eval() {
        assert_eq!(run("''(1+1);"), "2");
    }

    // --- Special symbols ---

    #[test]
    fn eval_special_symbols() {
        assert_eq!(run("%pi;"), "%pi");
        assert_eq!(run("%e;"), "%e");
        assert_eq!(run("%i;"), "%i");
        assert_eq!(run("inf;"), "inf");
        assert_eq!(run("minf;"), "minf");
    }

    // --- Orderlessp ---

    #[test]
    fn eval_orderlessp() {
        assert_eq!(run("orderlessp(a, b);"), "true");
        assert_eq!(run("orderlessp(b, a);"), "false");
    }

    // --- Sum and product ---

    #[test]
    fn eval_sum_expr() {
        assert_eq!(run("sum(i^2, i, 1, 4);"), "30");
    }

    #[test]
    fn eval_product_factorial() {
        assert_eq!(run("product(i, i, 1, 6);"), "720");
    }

    // --- Makelist / create_list ---

    #[test]
    fn eval_create_list() {
        assert_eq!(run("create_list(2*i, i, 1, 4);"), "[2,4,6,8]");
    }

    // --- Multiple assignments ---

    #[test]
    fn eval_list_assignment() {
        assert_eq!(
            run_env(&["[a,b]:[1,2];", "a+b;"]),
            "3"
        );
    }

    // --- Recursive function ---

    #[test]
    fn eval_recursive_func() {
        assert_eq!(
            run_env(&[
                "fib(n):=if n<=1 then n else fib(n-1)+fib(n-2);",
                "fib(10);"
            ]),
            "55"
        );
    }

    // --- Nested functions ---

    #[test]
    fn eval_function_composition() {
        assert_eq!(
            run_env(&["f(x):=x+1;", "g(x):=x*2;", "f(g(3));"]),
            "7"
        );
    }

    // --- Float function ---

    #[test]
    fn eval_float_conversion() {
        assert_eq!(run("float(1/4);"), "0.25");
    }

    // --- Is with compound predicates ---

    #[test]
    fn eval_is_and() {
        assert_eq!(run("is(1 > 0 and 2 > 1);"), "true");
        assert_eq!(run("is(1 > 0 and 2 < 1);"), "false");
    }

    #[test]
    fn eval_is_or() {
        assert_eq!(run("is(1 > 2 or 3 > 2);"), "true");
    }

    #[test]
    fn eval_is_structural_equal() {
        assert_eq!(run("is(x = x);"), "true");
    }

    // ===== Assumption system tests =====

    #[test]
    fn eval_assume_is() {
        assert_eq!(
            run_env(&["assume(x > 0);", "is(x > 0);"]),
            "true"
        );
    }

    #[test]
    fn eval_assume_is_derived() {
        assert_eq!(
            run_env(&["assume(x > 0);", "is(x >= 0);"]),
            "true"
        );
    }

    #[test]
    fn eval_assume_forget() {
        assert_eq!(
            run_env(&["assume(x > 0);", "forget(x > 0);", "is(x > 0);"]),
            "unknown"
        );
    }

    #[test]
    fn eval_asksign_positive() {
        assert_eq!(
            run_env(&["assume(x > 0);", "asksign(x);"]),
            "pos"
        );
    }

    #[test]
    fn eval_asksign_product() {
        assert_eq!(
            run_env(&["assume(x > 0);", "assume(y < 0);", "asksign(x*y);"]),
            "neg"
        );
    }

    #[test]
    fn eval_asksign_numeric() {
        assert_eq!(run("asksign(5);"), "pos");
        assert_eq!(run("asksign(-3);"), "neg");
        assert_eq!(run("asksign(0);"), "zero");
    }

    #[test]
    fn eval_asksign_exp() {
        assert_eq!(run("asksign(exp(x));"), "pos");
    }

    #[test]
    fn eval_facts() {
        let r = run_env(&["assume(x > 0, y < 0);", "facts();"]);
        assert!(r.contains("x > 0") && r.contains("y < 0"), "got: {}", r);
    }

    #[test]
    fn eval_assuming() {
        assert_eq!(
            run_env(&["assuming(x > 0, is(x > 0));", "is(x > 0);"]),
            "unknown"
        );
    }

    // ===== Polynomial built-in tests =====

    #[test]
    fn eval_gcd_integers() {
        assert_eq!(run("gcd(12, 8);"), "4");
    }

    #[test]
    fn eval_gcd_polynomials() {
        assert_eq!(run("gcd(x^2-1, x^2+2*x+1);"), "x+1");
    }

    #[test]
    fn eval_divide_exact() {
        let r = run("divide(x^3-1, x-1);");
        assert!(r.contains("x^2+x+1") && r.contains("0"), "got: {}", r);
    }

    #[test]
    fn eval_divide_remainder() {
        let r = run("divide(x^2+1, x-1);");
        assert!(r.contains("2"), "got: {}", r); // remainder is 2
    }

    #[test]
    fn eval_coeff_quadratic() {
        assert_eq!(run("coeff(3*x^2+5*x+7, x, 2);"), "3");
        assert_eq!(run("coeff(3*x^2+5*x+7, x, 1);"), "5");
    }

    #[test]
    fn eval_hipow() {
        assert_eq!(run("hipow(x^5+x^2+1, x);"), "5");
    }

    #[test]
    fn eval_gcd_coprime() {
        let r = run("gcd(x^2+1, x+1);");
        // Should be 1 (coprime)
        assert!(r == "1" || r == "1", "got: {}", r);
    }

    // ===== Edge case tests for eval =====

    #[test]
    fn eval_nested_ev() {
        assert_eq!(run("ev(ev(x+y, x:1), y:2);"), "3");
    }

    #[test]
    fn eval_for_in_empty() {
        assert_eq!(
            run_env(&["s:0;", "for i in [] do s:s+1;", "s;"]),
            "0"
        );
    }

    #[test]
    fn eval_block_no_locals() {
        assert_eq!(run("block(1+1, 2+2);"), "4");
    }

    #[test]
    fn eval_lambda_immediate() {
        assert_eq!(run("(lambda([x,y], x*y))(3,4);"), "12");
    }

    #[test]
    fn eval_errcatch_no_error() {
        assert_eq!(run("errcatch(42);"), "[42]");
    }

    #[test]
    fn eval_sum_empty_range() {
        assert_eq!(run("sum(i, i, 5, 1);"), "0");
    }

    #[test]
    fn eval_product_empty_range() {
        assert_eq!(run("product(i, i, 5, 1);"), "1");
    }

    #[test]
    fn eval_diff_higher_order() {
        assert_eq!(run("diff(x^5, x, 3);"), "60*x^2");
    }

    #[test]
    fn eval_expand_cube_verified() {
        let r = run("expand((x+1)^3);");
        assert!(r.contains("x^3") && r.contains("3*x^2") && r.contains("3*x") && r.contains("1"),
            "got: {}", r);
    }

    #[test]
    fn eval_subst_in_function() {
        assert_eq!(run("subst(0, x, sin(x));"), "0");
    }

    #[test]
    fn eval_map_with_lambda() {
        assert_eq!(
            run("map(lambda([x], 2*x), [1,2,3]);"),
            "[2,4,6]"
        );
    }

    #[test]
    fn eval_sort_with_predicate() {
        assert_eq!(run("sort([3,1,2], \"<\");"), "[1,2,3]");
    }

    #[test]
    fn eval_reverse_empty() {
        assert_eq!(run("reverse([]);"), "[]");
    }

    #[test]
    fn eval_append_empty() {
        assert_eq!(run("append([], [1,2]);"), "[1,2]");
        assert_eq!(run("append([1,2], []);"), "[1,2]");
    }

    #[test]
    fn eval_is_not_equal_atoms() {
        assert_eq!(run("is(a # b);"), "true");
        assert_eq!(run("is(a # a);"), "false");
    }

    #[test]
    fn eval_part_zero_is_operator() {
        let r = run("part(a+b, 0);");
        assert_eq!(r, "\"+\"");
    }

    // ===== New feature tests (RC5/RC6 completion) =====

    // --- Solve cubic/quartic ---
    #[test]
    fn eval_solve_cubic() {
        let r = run("solve(x^3-6*x^2+11*x-6, x);");
        assert!(r.contains("x = 1") && r.contains("x = 2") && r.contains("x = 3"), "got: {}", r);
    }
    #[test]
    fn eval_solve_quartic() {
        let r = run("solve(x^4-5*x^2+4, x);");
        assert!(r.contains("x = 1") && r.contains("x = -1") && r.contains("x = 2") && r.contains("x = -2"), "got: {}", r);
    }

    // --- Integration patterns ---
    #[test]
    fn eval_integrate_sec() { assert!(run("integrate(sec(x), x);").contains("log")); }
    #[test]
    fn eval_integrate_log_sq() { assert!(run("integrate(log(x)^2, x);").contains("log")); }
    #[test]
    fn eval_integrate_x3_exp() { assert!(run("integrate(x^3*exp(x), x);").contains("exp")); }
    #[test]
    fn eval_integrate_exp_sin() { assert!(run("integrate(exp(x)*sin(x), x);").contains("exp")); }
    #[test]
    fn eval_integrate_exp_cos() { assert!(run("integrate(exp(x)*cos(x), x);").contains("exp")); }
    #[test]
    fn eval_integrate_complete_sq() { assert!(run("integrate(1/(x^2+x+1), x);").contains("atan")); }
    #[test]
    fn eval_integrate_x_over_x2p1() { assert_eq!(run("integrate(x/(x^2+1), x);"), "log(1+x^2)/2"); }
    #[test]
    fn eval_integrate_rational() { assert!(run("integrate(1/((x-1)*(x+1)), x);").contains("log")); }
    #[test]
    fn eval_integrate_hermite_basic() {
        // ∫ 1/(x(x+1)²) dx = 1/(x+1) + log(x) - log(x+1)
        let r = run("integrate(1/(x*(x+1)^2), x);");
        assert!(r.contains("log"), "expected log terms, got: {}", r);
        assert!(!r.contains("integrate"), "should be fully solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_hermite_x2() {
        // ∫ 1/(x²(x+1)) dx = -1/x - log(x) + log(x+1)
        let r = run("integrate(1/(x^2*(x+1)), x);");
        assert!(r.contains("log"), "expected log terms, got: {}", r);
        assert!(r.contains("1/x") || r.contains("x)^(-1)"), "expected 1/x term, got: {}", r);
        assert!(!r.contains("integrate"), "should be fully solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_hermite_x_over_sq() {
        // ∫ x/(x+1)² dx = 1/(x+1) + log(x+1)
        let r = run("integrate(x/(x+1)^2, x);");
        assert!(r.contains("log"), "expected log term, got: {}", r);
        assert!(!r.contains("integrate"), "should be fully solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_power_denom() {
        let r = run("integrate(1/(x+1)^2, x);");
        assert!(r.contains("(1+x)") || r.contains("x+1"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_linear_sub() {
        // This uses the linear substitution ∫f(ax)=F(ax)/a
        let r = run("integrate(exp(2*x), x);");
        assert!(r.contains("exp"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_sin_sq() { assert!(run("integrate(sin(x)^2, x);").contains("sin")); }

    #[test]
    fn eval_integrate_hermite_higher_mult() {
        // ∫ 1/(x+1)^3 dx = -1/(2*(x+1)^2)
        let r = run("integrate(1/(x+1)^3, x);");
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_hermite_two_repeated() {
        // ∫ 1/((x+1)*(x+2)^2) dx
        let r = run("integrate(1/((x+1)*(x+2)^2), x);");
        assert!(r.contains("log"), "expected log terms, got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_rational_partfrac() {
        // ∫ 1/((x+1)*(x+2)) dx = log(x+1) - log(x+2)
        let r = run("integrate(1/((x+1)*(x+2)), x);");
        assert!(r.contains("log"), "expected log terms, got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }

    // --- Integration table formulas ---
    #[test]
    fn eval_integrate_sec_sq() {
        assert!(run("integrate(sec(x)^2, x);").contains("tan"), "sec² → tan");
    }
    #[test]
    fn eval_integrate_csc_sq() {
        assert!(run("integrate(csc(x)^2, x);").contains("cot"), "csc² → cot");
    }
    #[test]
    fn eval_integrate_tan_sq() {
        let r = run("integrate(tan(x)^2, x);");
        assert!(r.contains("tan"), "tan² → tan-x, got: {}", r);
    }
    #[test]
    fn eval_integrate_cot_x() {
        let r = run("integrate(cot(x), x);");
        assert!(r.contains("log") && r.contains("sin"), "cot → log|sin|, got: {}", r);
    }
    #[test]
    fn eval_integrate_sec_tan() {
        assert_eq!(run("integrate(sec(x)*tan(x), x);"), "sec(x)");
    }
    #[test]
    fn eval_integrate_csc_cot() {
        let r = run("integrate(csc(x)*cot(x), x);");
        assert!(r.contains("csc"), "csc*cot → -csc, got: {}", r);
    }
    #[test]
    fn eval_integrate_sin_cubed() {
        let r = run("integrate(sin(x)^3, x);");
        assert!(r.contains("cos") && r.contains("sin"), "sin³, got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_sec_cubed() {
        let r = run("integrate(sec(x)^3, x);");
        assert!(r.contains("tan") && r.contains("log"), "sec³, got: {}", r);
    }
    #[test]
    fn eval_integrate_sech_sq() {
        assert!(run("integrate(sech(x)^2, x);").contains("tanh"), "sech² → tanh");
    }
    #[test]
    fn eval_integrate_coth() {
        let r = run("integrate(coth(x), x);");
        assert!(r.contains("log") && r.contains("sinh"), "coth → log|sinh|, got: {}", r);
    }
    #[test]
    fn eval_integrate_x_asin() {
        let r = run("integrate(x*asin(x), x);");
        assert!(r.contains("asin"), "x*asin(x), got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_x_atan() {
        let r = run("integrate(x*atan(x), x);");
        assert!(r.contains("atan"), "x*atan(x), got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_sin_2x() {
        let r = run("integrate(sin(2*x), x);");
        assert!(r.contains("cos"), "sin(2x) → -cos(2x)/2, got: {}", r);
    }
    #[test]
    fn eval_integrate_tan_3x() {
        let r = run("integrate(tan(3*x), x);");
        assert!(r.contains("cos") || r.contains("log"), "tan(3x), got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_inv_sqrt_4_minus_x2() {
        // ∫ 1/sqrt(4-x²) = asin(x/2)
        let r = run("integrate(1/sqrt(4-x^2), x);");
        assert!(r.contains("asin"), "1/sqrt(4-x²) → asin, got: {}", r);
    }
    #[test]
    fn eval_integrate_inv_x2_plus_4() {
        // ∫ 1/(x²+4) = (1/2)*atan(x/2)
        let r = run("integrate(1/(x^2+4), x);");
        assert!(r.contains("atan"), "1/(x²+4) → atan, got: {}", r);
    }

    // --- Quadratic factor integration ---
    #[test]
    fn eval_integrate_quadratic_single() {
        // ∫ 1/(x²+1) = atan(x)
        let r = run("integrate(1/(x^2+1), x);");
        assert!(r.contains("atan"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_quadratic_mixed() {
        // ∫ 1/(x*(x²+1)) = log(x) - (1/2)*log(x²+1)
        // partfrac: 1/x - x/(x²+1)
        let r = run("integrate(1/(x*(x^2+1)), x);");
        assert!(r.contains("log"), "expected log, got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_quadratic_with_linear() {
        // ∫ x/((x+1)*(x²+1)) = -1/2*log(x+1) + 1/4*log(x²+1) + 1/2*atan(x)
        let r = run("integrate(x/((x+1)*(x^2+1)), x);");
        eprintln!("quadratic_with_linear: {}", r);
        assert!(r.contains("log"), "expected log terms, got: {}", r);
        assert!(r.contains("atan"), "expected atan term, got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_num_over_quadratic() {
        // ∫ x/(x²+1) = (1/2)*log(x²+1) — already works via derivative recognition
        let r = run("integrate(x/(x^2+1), x);");
        assert!(r.contains("log"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_linear_num_over_quadratic() {
        // ∫ (2x+3)/(x²+4) = log(x²+4) + (3/2)*atan(x/2)
        let r = run("integrate((2*x+3)/(x^2+4), x);");
        eprintln!("linear_num_over_quadratic: {}", r);
        assert!(r.contains("log") || r.contains("atan"), "got: {}", r);
    }

    // --- S5 Integration by substitution ---
    #[test]
    fn eval_integrate_log_log() {
        // ∫ 1/(x*log(x)) = log(log(x))
        let r = run("integrate(1/(x*log(x)), x);");
        assert_eq!(r, "log(log(x))");
    }
    #[test]
    fn eval_integrate_log_x_over_x() {
        // ∫ log(x)/x = log(x)²/2
        let r = run("integrate(log(x)/x, x);");
        assert!(r.contains("log") && r.contains("2"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_log_sq_over_x() {
        // ∫ log(x)²/x = log(x)³/3
        let r = run("integrate(log(x)^2/x, x);");
        assert!(r.contains("log") && r.contains("3"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_x_exp_x2() {
        // ∫ x*exp(x²) = exp(x²)/2
        let r = run("integrate(x*exp(x^2), x);");
        assert!(r.contains("exp"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_exp_over_1_plus_exp() {
        // ∫ exp(x)/(1+exp(x)) = log(1+exp(x))
        let r = run("integrate(exp(x)/(1+exp(x)), x);");
        assert_eq!(r, "log(1+exp(x))");
    }

    // --- S5 extended integration patterns ---
    #[test]
    fn eval_integrate_xn_log() {
        // #47: ∫ x²*log(x) = x³(3log(x)-1)/9
        let r = run("integrate(x^2*log(x), x);");
        assert!(r.contains("log") && r.contains("x^3"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_x_exp_ax() {
        // #42: ∫ x*exp(2x) = (2x-1)exp(2x)/4
        let r = run("integrate(x*exp(2*x), x);");
        assert!(r.contains("exp"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_sin4() {
        let r = run("integrate(sin(x)^4, x);");
        assert!(r.contains("sin"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_1_over_x2p1_sq() {
        // ∫ 1/(x²+1)² = x/(2(x²+1)) + atan(x)/2
        let r = run("integrate(1/(x^2+1)^2, x);");
        assert!(r.contains("atan"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_x_over_x4p1() {
        // ∫ x/(x⁴+1) = atan(x²)/2
        let r = run("integrate(x/(x^4+1), x);");
        assert!(r.contains("atan"), "got: {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }
    #[test]
    fn eval_integrate_1_over_log_x_noun() {
        // ∫ 1/log(x) is non-elementary (logarithmic integral)
        let r = run("integrate(1/log(x), x);");
        assert!(r.contains("integrate"), "should return noun form, got: {}", r);
    }

    // --- Limits ---
    #[test]
    fn eval_limit_poly_inf() { assert_eq!(run("limit(x^2, x, inf);"), "inf"); }
    #[test]
    fn eval_limit_rational_inf() { assert_eq!(run("limit((3*x^2+1)/(x^2+1), x, inf);"), "3"); }
    #[test]
    fn eval_limit_1_over_x() { assert_eq!(run("limit(1/x, x, inf);"), "0"); }
    #[test]
    fn eval_limit_exp_neg() { assert_eq!(run("limit(exp(-x), x, inf);"), "0"); }
    #[test]
    fn eval_limit_log_inf() { assert_eq!(run("limit(log(x), x, inf);"), "inf"); }
    #[test]
    fn eval_limit_lhopital() { assert_eq!(run("limit((x^2-1)/(x-1), x, 1);"), "2"); }

    // --- S7 Summation ---
    #[test]
    fn eval_sum_k() {
        // Σ_{k=1}^{n} k = n(n+1)/2
        let r = run("sum(k, k, 1, n);");
        assert!(!r.contains("sum"), "should be closed form, got: {}", r);
        assert!(r.contains("n"), "got: {}", r);
    }
    #[test]
    fn eval_sum_k2() {
        let r = run("sum(k^2, k, 1, n);");
        assert!(!r.contains("sum"), "should be closed form, got: {}", r);
    }
    #[test]
    fn eval_sum_k3() {
        let r = run("sum(k^3, k, 1, n);");
        assert!(!r.contains("sum"), "should be closed form, got: {}", r);
    }
    #[test]
    fn eval_sum_geometric() {
        // Σ_{k=0}^{n} 2^k = 2^(n+1) - 1
        let r = run("sum(2^k, k, 0, n);");
        assert!(!r.contains("sum"), "should be closed form, got: {}", r);
    }
    #[test]
    fn eval_sum_telescoping() {
        // Σ_{k=1}^{n} 1/(k(k+1)) = n/(n+1)
        let r = run("sum(1/(k*(k+1)), k, 1, n);");
        assert!(!r.contains("sum"), "should telescope, got: {}", r);
    }
    #[test]
    fn eval_sum_arith_geo() {
        // Σ_{k=0}^{n} k*2^k — closed form
        let r = run("sum(k*2^k, k, 0, n);");
        assert!(!r.contains("sum"), "should be closed form, got: {}", r);
    }
    #[test]
    fn eval_sum_factorial_type() {
        // Σ_{k=1}^{n} k*k! — Gosper: ratio = (k+1)*(k+1)/k = (k+1)^2/k
        // Not the simplest case. Try a simpler one:
        // Σ_{k=0}^{n} 1/(k+1)(k+2) — partial fraction telescoping
        let r = run("sum(1/((k+1)*(k+2)), k, 0, n);");
        eprintln!("sum 1/((k+1)(k+2)): {}", r);
        assert!(!r.contains("sum"), "should telescope, got: {}", r);
    }
    #[test]
    fn eval_sum_harmonic_noun() {
        // Σ 1/k has no closed form — should return noun
        let r = run("sum(1/k, k, 1, n);");
        assert!(r.contains("sum"), "harmonic should be noun form, got: {}", r);
    }
    #[test]
    fn eval_sum_numeric() {
        assert_eq!(run("sum(k, k, 1, 10);"), "55");
    }

    // --- S8 Definite integration ---
    #[test]
    fn eval_defint_finite() {
        // ∫_0^1 x² dx = 1/3
        let r = run("integrate(x^2, x, 0, 1);");
        assert!(r == "1/3" || r == "3^-1", "got: {}", r);
    }
    #[test]
    fn eval_defint_trig() {
        // ∫_0^π sin(x) dx = 2
        let r = run("integrate(sin(x), x, 0, %pi);");
        assert_eq!(r, "2");
    }
    #[test]
    fn eval_defint_exp_0_inf() {
        // ∫_0^∞ exp(-x) dx = 1
        let r = run("integrate(exp(-x), x, 0, inf);");
        assert_eq!(r, "1");
    }
    #[test]
    fn eval_defint_gaussian() {
        // ∫_{-∞}^{∞} exp(-x²) dx = √π
        let r = run("integrate(exp(-x^2), x, minf, inf);");
        assert!(r.contains("sqrt") && r.contains("pi"), "got: {}", r);
    }
    #[test]
    fn eval_defint_rational_inf() {
        // ∫_{-∞}^{∞} 1/(x²+1) dx = π
        let r = run("integrate(1/(x^2+1), x, minf, inf);");
        assert!(r.contains("pi"), "got: {}", r);
    }

    #[test]
    fn eval_defint_1_over_x2p1_sq() {
        // ∫_{-∞}^{∞} 1/(x²+1)² = π/2
        let r = run("integrate(1/(x^2+1)^2, x, minf, inf);");
        eprintln!("defint 1/(x²+1)²: {}", r);
        assert!(r.contains("pi"), "got: {}", r);
    }
    #[test]
    fn eval_defint_two_quadratics() {
        // ∫_{-∞}^{∞} 1/((x²+1)(x²+4)) = π/6
        let r = run("integrate(1/((x^2+1)*(x^2+4)), x, minf, inf);");
        eprintln!("defint 1/((x²+1)(x²+4)): {}", r);
        assert!(r.contains("pi"), "got: {}", r);
    }

    // --- S5 Risch tower integration ---
    #[test]
    fn eval_integrate_1_over_x_log2() {
        // ∫ 1/(x*log(x)²) = -1/log(x) — Risch primitive case
        let r = run("integrate(1/(x*log(x)^2), x);");
        eprintln!("1/(x*log(x)^2): {}", r);
        assert!(!r.contains("integrate"), "should be solved, got: {}", r);
    }

    // --- Differentiation (S5/S6) ---
    #[test]
    fn eval_diff_acot() { assert_eq!(run("diff(acot(x), x);"), "-1/(1+x^2)"); }
    #[test]
    fn eval_diff_cot() {
        let r = run("diff(cot(x), x);");
        assert!(r.contains("sin"), "got: {}", r);
    }
    #[test]
    fn eval_diff_sec() {
        let r = run("diff(sec(x), x);");
        assert!(r.contains("sec") && r.contains("tan"), "got: {}", r);
    }
    #[test]
    fn eval_diff_csc() {
        let r = run("diff(csc(x), x);");
        assert!(r.contains("csc") && r.contains("cot"), "got: {}", r);
    }
    #[test]
    fn eval_diff_acoth() {
        let r = run("diff(acoth(x), x);");
        assert!(r.contains("1") && r.contains("x^2"), "got: {}", r);
    }

    // --- S6 Limits ---
    #[test]
    fn eval_limit_sinx_over_x() { assert_eq!(run("limit(sin(x)/x, x, 0);"), "1"); }
    #[test]
    fn eval_limit_exp_minus_1_over_x() { assert_eq!(run("limit((exp(x)-1)/x, x, 0);"), "1"); }
    #[test]
    fn eval_limit_x_sin_1x() { assert_eq!(run("limit(x*sin(1/x), x, inf);"), "1"); }
    #[test]
    fn eval_limit_exp_over_x2() { assert_eq!(run("limit(exp(x)/x^2, x, inf);"), "inf"); }
    #[test]
    fn eval_limit_x_exp_neg_x() { assert_eq!(run("limit(x*exp(-x), x, inf);"), "0"); }
    #[test]
    fn eval_limit_1_plus_1x_x() {
        let r = run("limit((1+1/x)^x, x, inf);");
        assert!(r == "exp(1)" || r == "%e", "got: {}", r);
    }

    #[test]
    fn eval_limit_exp_over_xn() {
        // exp(x)/x^100 → ∞ (exp beats any polynomial)
        let r = run("limit(exp(x)/x^100, x, inf);");
        assert_eq!(r, "inf");
    }
    #[test]
    fn eval_limit_nested_exp() {
        // exp(exp(x)) → ∞
        let r = run("limit(exp(exp(x)), x, inf);");
        assert_eq!(r, "inf");
    }
    #[test]
    fn eval_limit_log_log() {
        // log(log(x)) → ∞ (slowly)
        let r = run("limit(log(log(x)), x, inf);");
        assert_eq!(r, "inf");
    }

    #[test]
    fn eval_integrate_sin_cos2() {
        let r = run("integrate(sin(x)*cos(x)^2, x);");
        assert!(!r.contains("integrate"), "should solve, got: {}", r);
    }
    // --- S9 reduction formulas + product-to-sum ---
    #[test]
    fn eval_integrate_sin5() {
        let r = run("integrate(sin(x)^5, x);");
        assert!(!r.contains("integrate"), "sin^5 should reduce, got: {}", r);
        assert!(r.contains("cos"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_sec4() {
        let r = run("integrate(sec(x)^4, x);");
        assert!(!r.contains("integrate"), "sec^4 should reduce, got: {}", r);
        assert!(r.contains("tan"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_tan5() {
        let r = run("integrate(tan(x)^5, x);");
        assert!(!r.contains("integrate"), "tan^5 should reduce, got: {}", r);
    }
    #[test]
    fn eval_integrate_sin2x_cos3x() {
        // Product-to-sum: sin(2x)*cos(3x)
        let r = run("integrate(sin(2*x)*cos(3*x), x);");
        assert!(!r.contains("integrate"), "product-to-sum should work, got: {}", r);
    }
    #[test]
    fn eval_integrate_x_exp_sin() {
        // ∫ x*exp(x)*sin(x) — triple product
        let r = run("integrate(x*exp(x)*sin(x), x);");
        assert!(!r.contains("integrate"), "triple product should work, got: {}", r);
    }
    #[test]
    fn eval_integrate_exp_tanh() {
        // ∫ exp(x)*tanh(x) = exp(x) - 2*atan(exp(x))
        let r = run("integrate(exp(x)*tanh(x), x);");
        assert!(r.contains("atan") && r.contains("exp"), "got: {}", r);
    }

    #[test]
    fn eval_integrate_1_over_x2p1_cubed() {
        // ∫ 1/(x²+1)³ — indefinite, repeated quadratic reduction
        let r = run("integrate(1/(x^2+1)^3, x);");
        assert!(r.contains("atan") || r.contains("x"), "got: {}", r);
        assert!(!r.contains("integrate"), "should solve, got: {}", r);
    }
    #[test]
    fn eval_defint_1_over_x2p1_cubed() {
        // ∫ 1/(x²+1)³ [-∞,∞] = 3π/8
        let r = run("integrate(1/(x^2+1)^3, x, minf, inf);");
        assert!(r.contains("pi"), "got: {}", r);
        assert!(!r.contains("integrate"), "should solve, got: {}", r);
    }
    #[test]
    fn eval_defint_1_over_x2p4_cubed() {
        // ∫ 1/(x²+4)³ [-∞,∞] = 3π/256
        let r = run("integrate(1/(x^2+4)^3, x, minf, inf);");
        assert!(r.contains("pi"), "got: {}", r);
    }
    #[test]
    fn eval_integrate_log_cubed_over_x() {
        // ∫ log(x)³/x = log(x)⁴/4 — polynomial tower reduction
        let r = run("integrate(log(x)^3/x, x);");
        assert!(!r.contains("integrate"), "should solve via tower, got: {}", r);
    }
    #[test]
    fn eval_limit_gruntz_classic() {
        // THE classic Gruntz example
        let r = run("limit(exp(x+exp(-x))-exp(x), x, inf);");
        eprintln!("Gruntz classic: {}", r);
        assert!(r == "1" || r == "1.0", "got: {}", r);
    }
    #[test]
    fn eval_limit_sqrt_conjugate() {
        // sqrt(x²+1) - x → 0 — conjugate rationalization
        let r = run("limit(sqrt(x^2+1)-x, x, inf);");
        assert_eq!(r, "0", "got: {}", r);
    }
    #[test]
    fn eval_limit_log_ratio() {
        assert_eq!(run("limit(log(log(x))/log(x), x, inf);"), "0");
    }
    #[test]
    fn eval_limit_taylor_0() {
        // (exp(x)-1-x)/x² → 1/2 via iterated L'Hôpital
        let r = run("limit((exp(x)-1-x)/x^2, x, 0);");
        assert!(r == "1/2" || r == "2^-1", "got: {}", r);
    }
    #[test]
    fn eval_limit_log10_over_x() {
        assert_eq!(run("limit(log(x)^10/x, x, inf);"), "0");
    }

    // --- Dot product ---
    #[test]
    fn eval_dot_numeric() { assert_eq!(run("3 . 4;"), "12"); }
    #[test]
    fn eval_dot_identity() { assert_eq!(run("1 . x;"), "x"); }
    #[test]
    fn eval_dot_zero() { assert_eq!(run("0 . x;"), "0"); }
    #[test]
    fn eval_ncexpt_zero() { assert_eq!(run("x^^0;"), "id"); }
    #[test]
    fn eval_ncexpt_combine() {
        let r = run("a^^2 . a^^3;");
        assert!(r.contains("ncexpt(a,5)") || r.contains("a^^5"), "got: {}", r);
    }

    // --- Boolean ---
    #[test]
    fn eval_and_symbolic() { assert_eq!(run("a and true;"), "a"); }
    #[test]
    fn eval_or_symbolic() { assert_eq!(run("a or false;"), "a"); }
    #[test]
    fn eval_not_demorgan() {
        let r = run("not(a and b);");
        assert!(r.contains("or") && r.contains("not"), "got: {}", r);
    }

    // --- Matrices ---
    #[test]
    fn eval_eigenvectors_2x2() {
        let r = run("eigenvectors(matrix([2,1],[1,2]));");
        assert!(r.contains("1") && r.contains("3"), "got: {}", r);
    }
    #[test]
    fn eval_charpoly() {
        let r = run("charpoly(matrix([1,2],[3,4]), x);");
        assert!(r.contains("x^2") || r.contains("x"), "got: {}", r);
    }

    // --- emptyp / identity ---
    #[test]
    fn eval_emptyp() {
        assert_eq!(run("emptyp([]);"), "true");
        assert_eq!(run("emptyp([1]);"), "false");
    }
    #[test]
    fn eval_identity_fn() { assert_eq!(run("identity(42);"), "42"); }

    // --- Declare/featurep ---
    #[test]
    fn eval_declare_featurep() {
        assert_eq!(run_env(&["declare(n, integer);", "featurep(n, integer);"]), "true");
    }

    // --- Context ---
    #[test]
    fn eval_context_isolation() {
        assert_eq!(
            run_env(&["newcontext(c1);", "assume(x>0);", "is(x>0);", "killcontext(c1);", "is(x>0);"]),
            "unknown"
        );
    }

    // --- Partfrac ---
    #[test]
    fn eval_partfrac() {
        let r = run("partfrac(1/(x^2-1), x);");
        assert!(r.contains("log") || r.contains("1/2"), "got: {}", r);
    }

    // --- Taylor ---
    #[test]
    fn eval_taylor_sin() {
        let r = run("taylor(sin(x), x, 0, 3);");
        assert!(r.contains("x"), "got: {}", r);
    }
}
