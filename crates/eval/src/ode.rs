use maxima_core::{Expr, Operator, SymbolId, resolve, intern};
use crate::simp::simplify;
use crate::helpers::{contains_var, subst, to_i64, to_f64};
use crate::eval::{meval, expand, diff_once};

pub(crate) fn eval_ode(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "ode2" => {
            if args.len() == 3 {
                return Some(ode2(&args[0], &args[1], &args[2], env));
            }
            None
        }
        "ic1" => {
            if args.len() == 4 {
                return Some(apply_ic1(&args[0], &args[1], &args[2], &args[3], env));
            }
            None
        }
        "ic2" => {
            if args.len() == 5 {
                return Some(apply_ic2(&args[0], &args[1], &args[2], &args[3], &args[4], env));
            }
            None
        }
        _ => None,
    }
}

fn ode2(eqn: &Expr, y_expr: &Expr, x_expr: &Expr, env: &mut crate::env::Environment) -> Expr {
    let (lhs, rhs) = match eqn {
        Expr::List { op: Operator::MEqual, args, .. } if args.len() == 2 => {
            (args[0].clone(), args[1].clone())
        }
        _ => (eqn.clone(), Expr::int(0)),
    };
    let f = simplify(&Expr::sub(lhs, rhs));

    // Detect order by checking for 'diff(y,x,2) vs 'diff(y,x)
    let dy = Expr::call("diff", vec![y_expr.clone(), x_expr.clone()]);
    let d2y = Expr::call("diff", vec![y_expr.clone(), x_expr.clone(), Expr::int(2)]);

    let has_d2y = contains_expr(&f, &d2y);
    let has_dy = contains_expr(&f, &dy);

    if has_d2y {
        return solve_second_order(&f, y_expr, x_expr, &dy, &d2y, env);
    }

    if has_dy {
        return solve_first_order(&f, y_expr, x_expr, &dy, env);
    }

    Expr::call("ode2", vec![eqn.clone(), y_expr.clone(), x_expr.clone()])
}

fn solve_first_order(f: &Expr, y: &Expr, x: &Expr, dy: &Expr, env: &mut crate::env::Environment) -> Expr {
    // Try to write as dy/dx = g(x,y)
    // f = 0 where f contains dy. Solve for dy: f = a*dy + b => dy = -b/a
    let a = coeff_of(f, dy);
    let b = simplify(&subst(&Expr::int(0), dy, f));

    if a != Expr::int(0) {
        let rhs = simplify(&Expr::neg(Expr::div(b.clone(), a.clone())));

        // Separable: dy/dx = g(x)*h(y)
        if let Some(sol) = try_separable(&rhs, y, x, env) { return sol; }

        // Linear: dy/dx + P(x)*y = Q(x)
        if let Some(sol) = try_linear_first_order(&rhs, y, x, env) { return sol; }
    }

    Expr::call("ode2", vec![Expr::add(f.clone(), Expr::int(0)), y.clone(), x.clone()])
}

fn try_separable(rhs: &Expr, y: &Expr, x: &Expr, env: &mut crate::env::Environment) -> Option<Expr> {
    // dy/dx = f(x)*g(y): ∫ 1/g(y) dy = ∫ f(x) dx + C
    let (fx, gy) = factor_separable(rhs, y, x)?;
    if gy == Expr::int(0) { return None; }

    let integrand_y = simplify(&Expr::pow(gy.clone(), Expr::int(-1)));
    let int_y = crate::integrate::table_integrate(&integrand_y, y);
    let int_x = crate::integrate::table_integrate(&fx, x);

    if int_y.to_string().contains("integrate") || int_x.to_string().contains("integrate") {
        return None;
    }

    let c = Expr::sym("%c");
    Some(Expr::List {
        op: Operator::MEqual,
        simplified: false,
        args: vec![simplify(&int_y), simplify(&Expr::add(int_x, c))],
    })
}

fn try_linear_first_order(rhs: &Expr, y: &Expr, x: &Expr, env: &mut crate::env::Environment) -> Option<Expr> {
    // dy/dx = rhs. Linear form: dy/dx + P(x)*y = Q(x)  =>  rhs = Q(x) - P(x)*y
    // Extract: rhs = A + B*y where A=Q(x), B=-P(x)
    let a_part = simplify(&subst(&Expr::int(0), y, rhs)); // rhs at y=0 = Q
    let b_part = simplify(&Expr::sub(rhs.clone(), a_part.clone()));

    // b_part should be B*y
    let b_coeff = coeff_of(&expand(&b_part), y);
    if b_coeff == Expr::int(0) || contains_var(&b_coeff, y) { return None; }

    // dy/dx = Q + B*y => dy/dx - B*y = Q => P = -B
    let p = simplify(&Expr::neg(b_coeff));
    let q = a_part;

    // Integrating factor: μ = exp(∫P dx)
    let int_p = crate::integrate::table_integrate(&p, x);
    if int_p.to_string().contains("integrate") { return None; }

    let mu = Expr::call("exp", vec![int_p.clone()]);
    // Solution: y = (1/μ) * (∫ μ*Q dx + C)
    let mu_q = simplify(&Expr::mul(mu.clone(), q));
    let int_mu_q = crate::integrate::table_integrate(&mu_q, x);
    if int_mu_q.to_string().contains("integrate") { return None; }

    let c = Expr::sym("%c");
    let sol = simplify(&Expr::mul(
        Expr::call("exp", vec![simplify(&Expr::neg(int_p))]),
        Expr::add(int_mu_q, c),
    ));
    Some(Expr::List {
        op: Operator::MEqual,
        simplified: false,
        args: vec![y.clone(), sol],
    })
}

fn solve_second_order(f: &Expr, y: &Expr, x: &Expr, dy: &Expr, d2y: &Expr, env: &mut crate::env::Environment) -> Expr {
    // Constant-coefficient: a*y'' + b*y' + c*y = 0
    let a = coeff_of(f, d2y);
    let b = coeff_of(f, dy);
    let residual = simplify(&subst(&Expr::int(0), d2y, &subst(&Expr::int(0), dy, f)));
    let c = coeff_of(&residual, y);
    let forcing = simplify(&subst(&Expr::int(0), y, &residual));

    if a == Expr::int(0) {
        return Expr::call("ode2", vec![Expr::add(f.clone(), Expr::int(0)), y.clone(), x.clone()]);
    }

    // Check constant coefficients (a, b, c don't depend on x)
    if contains_var(&a, x) || contains_var(&b, x) || contains_var(&c, x) {
        return Expr::call("ode2", vec![Expr::add(f.clone(), Expr::int(0)), y.clone(), x.clone()]);
    }

    if forcing == Expr::int(0) {
        return solve_const_coeff_homogeneous(&a, &b, &c, y, x);
    }

    // Non-homogeneous: try undetermined coefficients
    let homogeneous = solve_const_coeff_homogeneous(&a, &b, &c, y, x);
    let forcing_neg = simplify(&Expr::neg(forcing.clone()));
    if let Some(particular) = try_undetermined_coefficients(&a, &b, &c, &forcing_neg, x, env) {
        if let Expr::List { op: Operator::MEqual, args: hsides, .. } = &homogeneous {
            let full = simplify(&Expr::add(hsides[1].clone(), particular));
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), full] };
        }
    }

    Expr::call("ode2", vec![Expr::add(f.clone(), Expr::int(0)), y.clone(), x.clone()])
}

fn solve_const_coeff_homogeneous(a: &Expr, b: &Expr, c: &Expr, y: &Expr, x: &Expr) -> Expr {
    // Discriminant: b² - 4ac
    let disc = simplify(&Expr::sub(
        Expr::mul(b.clone(), b.clone()),
        Expr::mul(Expr::int(4), Expr::mul(a.clone(), c.clone())),
    ));

    let k1 = Expr::sym("%k1");
    let k2 = Expr::sym("%k2");
    let two_a = simplify(&Expr::mul(Expr::int(2), a.clone()));

    if let Some(d) = to_f64(&disc) {
        if d.abs() < 1e-15 {
            // Repeated root: r = -b/(2a)
            let r = simplify(&Expr::div(Expr::neg(b.clone()), two_a));
            let sol = simplify(&Expr::mul(
                Expr::call("exp", vec![Expr::mul(r.clone(), x.clone())]),
                Expr::add(k1, Expr::mul(k2, x.clone())),
            ));
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), sol] };
        } else if d > 0.0 {
            // Two real roots
            let sqrt_d = simplify(&Expr::call("sqrt", vec![disc.clone()]));
            let r1 = simplify(&Expr::div(Expr::add(Expr::neg(b.clone()), sqrt_d.clone()), two_a.clone()));
            let r2 = simplify(&Expr::div(Expr::sub(Expr::neg(b.clone()), sqrt_d), two_a));
            let sol = simplify(&Expr::add(
                Expr::mul(k1, Expr::call("exp", vec![Expr::mul(r1, x.clone())])),
                Expr::mul(k2, Expr::call("exp", vec![Expr::mul(r2, x.clone())])),
            ));
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), sol] };
        } else {
            // Complex roots: r = α ± βi
            let alpha = simplify(&Expr::div(Expr::neg(b.clone()), two_a.clone()));
            let beta = simplify(&Expr::div(
                Expr::call("sqrt", vec![simplify(&Expr::neg(disc))]),
                two_a,
            ));
            let sol = simplify(&Expr::mul(
                Expr::call("exp", vec![Expr::mul(alpha, x.clone())]),
                Expr::add(
                    Expr::mul(k1, Expr::call("cos", vec![Expr::mul(beta.clone(), x.clone())])),
                    Expr::mul(k2, Expr::call("sin", vec![Expr::mul(beta, x.clone())])),
                ),
            ));
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), sol] };
        }
    }

    Expr::call("ode2", vec![Expr::int(0), y.clone(), x.clone()])
}

fn try_undetermined_coefficients(
    a: &Expr, b: &Expr, c: &Expr, g: &Expr, x: &Expr,
    env: &mut crate::env::Environment,
) -> Option<Expr> {
    // g(x) is the forcing function. Try ansatz based on form of g.
    let (a_f, b_f, c_f) = (to_f64(a)?, to_f64(b)?, to_f64(c)?);

    // g = polynomial: try polynomial of same degree
    if let Some(var_id) = match x { Expr::Symbol(id) => Some(*id), _ => None } {
        if let Some(gp) = maxima_poly::expr_to_poly(&expand(g), var_id) {
            let deg = gp.degree().unwrap_or(0);
            return try_poly_ansatz(a_f, b_f, c_f, &gp, x, var_id, deg);
        }
    }

    // g = A*sin(w*x) or A*cos(w*x): try P*cos(wx) + Q*sin(wx)
    if let Some((w, is_sin, amp)) = extract_sincos(g, x) {
        return try_sincos_ansatz(a_f, b_f, c_f, w, &amp, is_sin, x);
    }

    // g = A*exp(k*x): try B*exp(kx), or Bx*exp(kx) if resonance
    if let Some((k, amp)) = extract_exp_forcing(g, x) {
        return try_exp_ansatz(a_f, b_f, c_f, k, &amp, x);
    }

    None
}

fn try_sincos_ansatz(a: f64, b: f64, c: f64, w: f64, amp: &Expr, is_sin: bool, x: &Expr) -> Option<Expr> {
    // y'' + b*y' + c*y = g  where g involves sin(wx) or cos(wx)
    // Ansatz: yp = P*cos(wx) + Q*sin(wx)
    // yp'' = -w²P*cos - w²Q*sin, yp' = -wP*sin + wQ*cos
    // Substituting: (-w²P + bwQ + cP)*cos + (-w²Q - bwP + cQ)*sin = g
    // If g = sin(wx): P_coeff = 0, Q_coeff = 1
    // If g = cos(wx): P_coeff = 1, Q_coeff = 0
    let alpha = c - w * w; // coefficient for P in cos equation
    let beta = b * w;       // coefficient for Q in cos equation (and -P in sin equation)
    // System: alpha*P + beta*Q = (cos coefficient of g)
    //        -beta*P + alpha*Q = (sin coefficient of g)
    let det = alpha * alpha + beta * beta;
    if det.abs() < 1e-15 { return None; } // resonance — would need x*ansatz

    let (gc, gs) = if is_sin { (0.0, 1.0) } else { (1.0, 0.0) };
    let p_val = (alpha * gc + beta * gs) / det;
    let q_val = (-beta * gc + alpha * gs) / det;

    let wx = simplify(&Expr::mul(float_or_int(w), x.clone()));
    let mut terms = Vec::new();
    if p_val.abs() > 1e-15 {
        terms.push(simplify(&Expr::mul(
            Expr::mul(amp.clone(), float_or_int(p_val)),
            Expr::call("cos", vec![wx.clone()]))));
    }
    if q_val.abs() > 1e-15 {
        terms.push(simplify(&Expr::mul(
            Expr::mul(amp.clone(), float_or_int(q_val)),
            Expr::call("sin", vec![wx]))));
    }
    if terms.is_empty() { return None; }
    if terms.len() == 1 { return Some(terms.pop().unwrap()); }
    Some(simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }))
}

fn try_exp_ansatz(a: f64, b: f64, c: f64, k: f64, amp: &Expr, x: &Expr) -> Option<Expr> {
    let char_val = a * k * k + b * k + c;
    let kx = simplify(&Expr::mul(float_or_int(k), x.clone()));
    let exp_kx = Expr::call("exp", vec![kx.clone()]);
    if char_val.abs() > 1e-15 {
        let coeff = 1.0 / char_val;
        Some(simplify(&Expr::mul(Expr::mul(amp.clone(), float_or_int(coeff)), exp_kx)))
    } else {
        // Resonance: try x*B*exp(kx)
        let char_deriv = 2.0 * a * k + b;
        if char_deriv.abs() > 1e-15 {
            let coeff = 1.0 / char_deriv;
            Some(simplify(&Expr::mul(Expr::mul(amp.clone(), float_or_int(coeff)),
                Expr::mul(x.clone(), exp_kx))))
        } else { None }
    }
}

fn try_poly_ansatz(a: f64, b: f64, c: f64, gp: &maxima_poly::Poly, x: &Expr, var: maxima_core::SymbolId, deg: u32) -> Option<Expr> {
    if c.abs() < 1e-15 { return None; }
    // For constant forcing: yp = g/c
    if deg == 0 {
        let g0 = match gp.constant_term() {
            maxima_poly::Coeff::Int(n) => n as f64,
            maxima_poly::Coeff::Rat(n, d) => n as f64 / d as f64,
        };
        return Some(float_or_int(g0 / c));
    }
    None
}

fn extract_sincos(g: &Expr, x: &Expr) -> Option<(f64, bool, Expr)> {
    // Match A*sin(w*x) or A*cos(w*x) or sin(w*x) or cos(w*x)
    fn try_single(e: &Expr, x: &Expr) -> Option<(f64, bool)> {
        if let Expr::List { op: Operator::Named(id), args, .. } = e {
            let fname = resolve(*id);
            if (fname == "sin" || fname == "cos") && args.len() == 1 {
                if let Some(w) = crate::helpers::to_f64(&args[0].clone()) {
                    return None; // not w*x form
                }
                if args[0] == *x { return Some((1.0, fname == "sin")); }
                if let Expr::List { op: Operator::MTimes, args: ma, .. } = &args[0] {
                    if ma.len() == 2 {
                        if ma[1] == *x { if let Some(w) = to_f64(&ma[0]) { return Some((w, fname == "sin")); } }
                        if ma[0] == *x { if let Some(w) = to_f64(&ma[1]) { return Some((w, fname == "sin")); } }
                    }
                }
            }
        }
        None
    }
    if let Some((w, is_sin)) = try_single(g, x) {
        return Some((w, is_sin, Expr::int(1)));
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = g {
        for (i, a) in args.iter().enumerate() {
            if let Some((w, is_sin)) = try_single(a, x) {
                let coeff: Vec<Expr> = args.iter().enumerate().filter(|(j,_)| *j != i).map(|(_,e)| e.clone()).collect();
                let amp = if coeff.len() == 1 { coeff[0].clone() }
                    else { Expr::List { op: Operator::MTimes, simplified: false, args: coeff } };
                return Some((w, is_sin, amp));
            }
        }
    }
    None
}

fn extract_exp_forcing(g: &Expr, x: &Expr) -> Option<(f64, Expr)> {
    fn try_exp(e: &Expr, x: &Expr) -> Option<f64> {
        if let Expr::List { op: Operator::Named(id), args, .. } = e {
            if resolve(*id) == "exp" && args.len() == 1 {
                if args[0] == *x { return Some(1.0); }
                if let Expr::List { op: Operator::MTimes, args: ma, .. } = &args[0] {
                    if ma.len() == 2 {
                        if ma[1] == *x { return to_f64(&ma[0]); }
                        if ma[0] == *x { return to_f64(&ma[1]); }
                    }
                }
            }
        }
        None
    }
    if let Some(k) = try_exp(g, x) { return Some((k, Expr::int(1))); }
    if let Expr::List { op: Operator::MTimes, args, .. } = g {
        for (i, a) in args.iter().enumerate() {
            if let Some(k) = try_exp(a, x) {
                let coeff: Vec<Expr> = args.iter().enumerate().filter(|(j,_)| *j != i).map(|(_,e)| e.clone()).collect();
                let amp = if coeff.len() == 1 { coeff[0].clone() }
                    else { Expr::List { op: Operator::MTimes, simplified: false, args: coeff } };
                return Some((k, amp));
            }
        }
    }
    None
}

fn float_or_int(v: f64) -> Expr {
    let i = v.round() as i64;
    if (v - i as f64).abs() < 1e-12 { Expr::int(i) }
    else {
        let denom = 1000i64;
        let numer = (v * denom as f64).round() as i64;
        let g = gcd_u64(numer.unsigned_abs(), denom as u64) as i64;
        if g > 1 { Expr::Rational { num: numer/g, den: denom/g } }
        else { Expr::Float(v) }
    }
}

fn gcd_u64(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd_u64(b, a % b) } }

fn apply_ic1(sol: &Expr, x_eq: &Expr, y_eq: &Expr, _dummy: &Expr, env: &mut crate::env::Environment) -> Expr {
    // ic1(sol, x=a, y=b) — actually takes 3 args after sol
    // But Maxima's ic1 takes: ic1(sol, x=a, y=b)
    // sol is y = f(x, %c), substitute x=a, y=b, solve for %c
    if let (
        Expr::List { op: Operator::MEqual, args: sol_sides, .. },
        Expr::List { op: Operator::MEqual, args: x_sides, .. },
        Expr::List { op: Operator::MEqual, args: y_sides, .. },
    ) = (sol, x_eq, y_eq) {
        let rhs = &sol_sides[1];
        let x_val = &x_sides[1];
        let y_val = &y_sides[1];
        let x_var = &x_sides[0];

        let rhs_at_x = subst(x_val, x_var, rhs);
        let c_eq = simplify(&Expr::sub(y_val.clone(), rhs_at_x));
        // c_eq = y_val - f(x_val, %c) = 0 → solve for %c
        let c_sym = Expr::sym("%c");
        let c_coeff = coeff_of(&expand(&c_eq), &c_sym);
        let c_const = simplify(&subst(&Expr::int(0), &c_sym, &c_eq));
        if c_coeff != Expr::int(0) {
            let c_val = simplify(&Expr::neg(Expr::div(c_const, c_coeff)));
            let result_rhs = simplify(&subst(&c_val, &c_sym, rhs));
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![sol_sides[0].clone(), result_rhs] };
        }
    }
    Expr::call("ic1", vec![sol.clone(), x_eq.clone(), y_eq.clone()])
}

fn apply_ic2(sol: &Expr, x_eq: &Expr, y_eq: &Expr, dy_eq: &Expr, _dummy: &Expr, env: &mut crate::env::Environment) -> Expr {
    Expr::call("ic2", vec![sol.clone(), x_eq.clone(), y_eq.clone(), dy_eq.clone()])
}

fn coeff_of(expr: &Expr, term: &Expr) -> Expr {
    let without = simplify(&subst(&Expr::int(0), term, expr));
    let with = simplify(&subst(&Expr::int(1), term, expr));
    simplify(&Expr::sub(with, without))
}

fn factor_separable(expr: &Expr, y: &Expr, x: &Expr) -> Option<(Expr, Expr)> {
    // Check if expr = f(x) * g(y) or just f(x) or just g(y)
    if !contains_var(expr, y) {
        return Some((expr.clone(), Expr::int(1)));
    }
    if !contains_var(expr, x) {
        return Some((Expr::int(1), expr.clone()));
    }
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut x_parts = Vec::new();
        let mut y_parts = Vec::new();
        for a in args {
            if !contains_var(a, y) {
                x_parts.push(a.clone());
            } else if !contains_var(a, x) {
                y_parts.push(a.clone());
            } else {
                return None;
            }
        }
        let fx = if x_parts.is_empty() { Expr::int(1) }
            else if x_parts.len() == 1 { x_parts.pop().unwrap() }
            else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: x_parts }) };
        let gy = if y_parts.is_empty() { Expr::int(1) }
            else if y_parts.len() == 1 { y_parts.pop().unwrap() }
            else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: y_parts }) };
        Some((fx, gy))
    } else {
        None
    }
}

fn contains_expr(haystack: &Expr, needle: &Expr) -> bool {
    if haystack == needle { return true; }
    if let Expr::List { args, .. } = haystack {
        args.iter().any(|a| contains_expr(a, needle))
    } else {
        false
    }
}
