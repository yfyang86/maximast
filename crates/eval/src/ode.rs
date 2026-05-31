use maxima_core::{Expr, Operator, SymbolId, resolve, intern};
use crate::simp::simplify;
use crate::helpers::{contains_var, subst, to_i64, to_f64};
use crate::eval::{meval, expand, diff_once, ratsimp};

pub(crate) fn eval_ode(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "ode2" => {
            if args.len() == 3 {
                return Some(ode2(&args[0], &args[1], &args[2], env));
            }
            None
        }
        "ic1" => {
            if args.len() == 3 {
                return Some(apply_ic1(&args[0], &args[1], &args[2], env));
            }
            None
        }
        "ic2" => {
            if args.len() == 4 {
                return Some(apply_ic2(&args[0], &args[1], &args[2], &args[3], env));
            }
            None
        }
        "bc2" => {
            if args.len() == 5 {
                return Some(apply_bc2(&args[0], &args[1], &args[2], &args[3], &args[4], env));
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
        return clean_solution(&solve_const_coeff_homogeneous(&a, &b, &c, y, x), env);
    }

    // Non-homogeneous. The RHS forcing g(x) is -forcing (f = lhs - rhs).
    let homogeneous = solve_const_coeff_homogeneous(&a, &b, &c, y, x);
    let g = simplify(&Expr::neg(forcing.clone()));

    // 1) Undetermined coefficients (closed-form, exact).
    if let Some(particular) = try_undetermined_coefficients(&a, &b, &c, &g, x, env) {
        if verify_particular(&a, &b, &c, &g, &particular, x) {
            if let Expr::List { op: Operator::MEqual, args: hsides, .. } = &homogeneous {
                let full = simplify(&Expr::add(hsides[1].clone(), particular));
                return clean_solution(
                    &Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), full] }, env);
            }
        }
    }

    // 2) Variation of parameters (general). Always numerically verified
    //    before use — the integrator may silently return a wrong result,
    //    so an unverified particular solution is discarded as a noun form.
    if let Some(particular) = try_variation_of_parameters(&homogeneous, &a, &g, x, env) {
        if verify_particular(&a, &b, &c, &g, &particular, x) {
            if let Expr::List { op: Operator::MEqual, args: hsides, .. } = &homogeneous {
                let full = simplify(&Expr::add(hsides[1].clone(), particular));
                return clean_solution(
                    &Expr::List { op: Operator::MEqual, simplified: false, args: vec![y.clone(), full] }, env);
            }
        }
    }

    Expr::call("ode2", vec![Expr::add(f.clone(), Expr::int(0)), y.clone(), x.clone()])
}

/// meval the RHS of a `y = ...` solution so leftover symbolic-builder
/// artifacts (exp(0), 2*x/2, ...) collapse before display.
fn clean_solution(sol: &Expr, env: &mut crate::env::Environment) -> Expr {
    if let Expr::List { op: Operator::MEqual, args, .. } = sol {
        if args.len() == 2 {
            let rhs = meval(&args[1], env);
            return Expr::List { op: Operator::MEqual, simplified: false,
                args: vec![args[0].clone(), rhs] };
        }
    }
    sol.clone()
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

fn try_poly_ansatz(a: f64, b: f64, c: f64, gp: &maxima_poly::Poly, x: &Expr, _var: maxima_core::SymbolId, deg: u32) -> Option<Expr> {
    // Solve a*yp'' + b*yp' + c*yp = g(x) for polynomial g via matching
    // coefficients. The coefficient of x^j in L[yp] (yp = Σ p_k x^k) is
    //   c*p_j + b*(j+1)*p_{j+1} + a*(j+1)(j+2)*p_{j+2}.
    // When the lowest acting term vanishes (c=0, or c=b=0) the ansatz
    // degree is raised so the system stays solvable.
    let n = deg as usize;
    let g: Vec<f64> = (0..=n).map(|i| poly_coeff_f64(gp, i as u32)).collect();

    let total = if c.abs() > 1e-12 { n }
        else if b.abs() > 1e-12 { n + 1 }
        else { n + 2 };
    let mut p = vec![0.0f64; total + 3];

    if c.abs() > 1e-12 {
        for j in (0..=n).rev() {
            let known = b * ((j + 1) as f64) * p[j + 1]
                + a * ((j + 1) as f64) * ((j + 2) as f64) * p[j + 2];
            p[j] = (g[j] - known) / c;
        }
    } else if b.abs() > 1e-12 {
        for j in (0..=n).rev() {
            let known = a * ((j + 1) as f64) * ((j + 2) as f64) * p[j + 2];
            p[j + 1] = (g[j] - known) / (b * ((j + 1) as f64));
        }
    } else {
        if a.abs() < 1e-12 { return None; }
        for j in (0..=n).rev() {
            p[j + 2] = g[j] / (a * ((j + 1) as f64) * ((j + 2) as f64));
        }
    }

    let mut terms = Vec::new();
    for (j, &pj) in p.iter().enumerate().take(total + 1) {
        if pj.abs() <= 1e-12 { continue; }
        let coeff = float_or_int(pj);
        let term = if j == 0 { coeff }
            else if j == 1 { simplify(&Expr::mul(coeff, x.clone())) }
            else { simplify(&Expr::mul(coeff, Expr::pow(x.clone(), Expr::int(j as i64)))) };
        terms.push(term);
    }
    if terms.is_empty() { return Some(Expr::int(0)); }
    if terms.len() == 1 { return Some(terms.pop().unwrap()); }
    Some(simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }))
}

fn poly_coeff_f64(p: &maxima_poly::Poly, exp: u32) -> f64 {
    p.terms.iter()
        .find(|(e, _)| *e == exp)
        .map(|(_, c)| match c {
            maxima_poly::Coeff::Int(n) => *n as f64,
            maxima_poly::Coeff::Rat(n, d) => *n as f64 / *d as f64,
        })
        .unwrap_or(0.0)
}

fn extract_basis(homogeneous: &Expr, env: &mut crate::env::Environment) -> Option<(Expr, Expr)> {
    if let Expr::List { op: Operator::MEqual, args, .. } = homogeneous {
        let rhs = &args[1];
        let k1 = Expr::sym("%k1");
        let k2 = Expr::sym("%k2");
        // meval folds residual arithmetic the symbolic builder leaves behind
        // (e.g. exp(0)→1, cos(2*x/2)→cos(x)) so the integrator can recognise
        // the basis functions.
        let y1 = meval(&subst(&Expr::int(1), &k1, &subst(&Expr::int(0), &k2, rhs)), env);
        let y2 = meval(&subst(&Expr::int(0), &k1, &subst(&Expr::int(1), &k2, rhs)), env);
        if y1 == Expr::int(0) || y2 == Expr::int(0) { return None; }
        return Some((y1, y2));
    }
    None
}

fn try_variation_of_parameters(homogeneous: &Expr, a: &Expr, g: &Expr, x: &Expr,
    env: &mut crate::env::Environment) -> Option<Expr> {
    // y1, y2 span the homogeneous solution; g(x) is the RHS forcing.
    // Normalise to y'' + ... = g/a, then
    //   yp = -y1 ∫ y2*(g/a)/W dx + y2 ∫ y1*(g/a)/W dx,   W = y1 y2' - y2 y1'.
    let (y1, y2) = extract_basis(homogeneous, env)?;
    let gn = simplify(&Expr::div(g.clone(), a.clone()));

    let y1p = diff_once(&y1, x);
    let y2p = diff_once(&y2, x);
    let w = simplify(&Expr::sub(
        Expr::mul(y1.clone(), y2p),
        Expr::mul(y2.clone(), y1p),
    ));
    if w == Expr::int(0) { return None; }

    let i1_integrand = simplify(&Expr::div(Expr::mul(y2.clone(), gn.clone()), w.clone()));
    let i2_integrand = simplify(&Expr::div(Expr::mul(y1.clone(), gn), w));
    let i1 = crate::integrate::table_integrate(&i1_integrand, x);
    let i2 = crate::integrate::table_integrate(&i2_integrand, x);
    if i1.to_string().contains("integrate") || i2.to_string().contains("integrate") {
        return None;
    }

    let yp = simplify(&Expr::add(
        Expr::neg(Expr::mul(y1, i1)),
        Expr::mul(y2, i2),
    ));
    Some(yp)
}

/// Numerically confirm yp satisfies a*yp'' + b*yp' + c*yp = g at several
/// sample points. Guards against silently-wrong symbolic integration.
fn verify_particular(a: &Expr, b: &Expr, c: &Expr, g: &Expr, yp: &Expr, x: &Expr) -> bool {
    let var = match x { Expr::Symbol(id) => *id, _ => return false };
    let yp1 = diff_once(yp, x);
    let yp2 = diff_once(&yp1, x);
    let samples = [0.3f64, 0.8, 1.4, 2.1, -0.7];
    let mut checked = 0;
    for &v in &samples {
        let (Some(av), Some(bv), Some(cv)) =
            (numeric_eval(a, var, v), numeric_eval(b, var, v), numeric_eval(c, var, v))
            else { continue };
        let (Some(y0), Some(y1), Some(y2), Some(gv)) = (
            numeric_eval(yp, var, v),
            numeric_eval(&yp1, var, v),
            numeric_eval(&yp2, var, v),
            numeric_eval(g, var, v),
        ) else { continue };
        let lhs = av * y2 + bv * y1 + cv * y0;
        if !(lhs - gv).abs().is_finite() { continue; }
        if (lhs - gv).abs() > 1e-6 * (1.0 + gv.abs()) { return false; }
        checked += 1;
    }
    checked >= 2
}

fn numeric_eval(e: &Expr, var: maxima_core::SymbolId, val: f64) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        Expr::Symbol(id) => {
            if *id == var { return Some(val); }
            match resolve(*id).as_str() {
                "%pi" => Some(std::f64::consts::PI),
                "%e" => Some(std::f64::consts::E),
                _ => None,
            }
        }
        Expr::List { op, args, .. } => match op {
            Operator::MPlus => {
                let mut s = 0.0;
                for a in args { s += numeric_eval(a, var, val)?; }
                Some(s)
            }
            Operator::MTimes => {
                let mut p = 1.0;
                for a in args { p *= numeric_eval(a, var, val)?; }
                Some(p)
            }
            Operator::MExpt if args.len() == 2 => {
                let base = numeric_eval(&args[0], var, val)?;
                let exp = numeric_eval(&args[1], var, val)?;
                Some(base.powf(exp))
            }
            Operator::Named(id) if args.len() == 1 => {
                let a = numeric_eval(&args[0], var, val)?;
                match resolve(*id).as_str() {
                    "sin" => Some(a.sin()),
                    "cos" => Some(a.cos()),
                    "tan" => Some(a.tan()),
                    "exp" => Some(a.exp()),
                    "log" => Some(a.ln()),
                    "sqrt" => Some(a.sqrt()),
                    "sinh" => Some(a.sinh()),
                    "cosh" => Some(a.cosh()),
                    "tanh" => Some(a.tanh()),
                    "abs" => Some(a.abs()),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
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

fn apply_ic1(sol: &Expr, x_eq: &Expr, y_eq: &Expr, env: &mut crate::env::Environment) -> Expr {
    // ic1(sol, x=a, y=b):
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
            let result_rhs = meval(&subst(&c_val, &c_sym, rhs), env);
            return Expr::List { op: Operator::MEqual, simplified: false, args: vec![sol_sides[0].clone(), result_rhs] };
        }
    }
    Expr::call("ic1", vec![sol.clone(), x_eq.clone(), y_eq.clone()])
}

fn apply_ic2(sol: &Expr, x_eq: &Expr, y_eq: &Expr, dy_eq: &Expr, env: &mut crate::env::Environment) -> Expr {
    // ic2(sol, x=x0, y=y0, 'diff(y,x)=dy0): solve for %k1,%k2.
    if let (
        Expr::List { op: Operator::MEqual, args: sol_s, .. },
        Expr::List { op: Operator::MEqual, args: x_s, .. },
        Expr::List { op: Operator::MEqual, args: y_s, .. },
        Expr::List { op: Operator::MEqual, args: dy_s, .. },
    ) = (sol, x_eq, y_eq, dy_eq) {
        let rhs = &sol_s[1];
        let x_var = &x_s[0];
        let x0 = &x_s[1];
        let yprime = diff_once(rhs, x_var);
        let rhs_at = simplify(&subst(x0, x_var, rhs));
        let yp_at = simplify(&subst(x0, x_var, &yprime));
        if let Some(rhs_final) = solve_two_consts(
            rhs, (&rhs_at, &y_s[1]), (&yp_at, &dy_s[1]), env) {
            return Expr::List { op: Operator::MEqual, simplified: false,
                args: vec![sol_s[0].clone(), rhs_final] };
        }
    }
    Expr::call("ic2", vec![sol.clone(), x_eq.clone(), y_eq.clone(), dy_eq.clone()])
}

fn apply_bc2(sol: &Expr, x_eq1: &Expr, y_eq1: &Expr, x_eq2: &Expr, y_eq2: &Expr,
    env: &mut crate::env::Environment) -> Expr {
    // bc2(sol, x=x0, y=y0, x=x1, y=y1): two boundary points, solve %k1,%k2.
    if let (
        Expr::List { op: Operator::MEqual, args: sol_s, .. },
        Expr::List { op: Operator::MEqual, args: x0_s, .. },
        Expr::List { op: Operator::MEqual, args: y0_s, .. },
        Expr::List { op: Operator::MEqual, args: x1_s, .. },
        Expr::List { op: Operator::MEqual, args: y1_s, .. },
    ) = (sol, x_eq1, y_eq1, x_eq2, y_eq2) {
        let rhs = &sol_s[1];
        let x_var = &x0_s[0];
        let rhs_at0 = simplify(&subst(&x0_s[1], x_var, rhs));
        let rhs_at1 = simplify(&subst(&x1_s[1], x_var, rhs));
        if let Some(rhs_final) = solve_two_consts(
            rhs, (&rhs_at0, &y0_s[1]), (&rhs_at1, &y1_s[1]), env) {
            return Expr::List { op: Operator::MEqual, simplified: false,
                args: vec![sol_s[0].clone(), rhs_final] };
        }
    }
    Expr::call("bc2", vec![sol.clone(), x_eq1.clone(), y_eq1.clone(), x_eq2.clone(), y_eq2.clone()])
}

/// Solve the 2x2 linear system in %k1,%k2 given two constraints
/// `lhs_i = rhs_i` (each lhs linear in %k1,%k2), then substitute the
/// solved constants back into `expr`.
fn solve_two_consts(expr: &Expr, c1: (&Expr, &Expr), c2: (&Expr, &Expr),
    env: &mut crate::env::Environment) -> Option<Expr> {
    let k1 = Expr::sym("%k1");
    let k2 = Expr::sym("%k2");
    // meval folds boundary values like cos(0), sin(%pi/2), exp(0).
    let e1 = expand(&meval(&Expr::sub(c1.0.clone(), c1.1.clone()), env));
    let e2 = expand(&meval(&Expr::sub(c2.0.clone(), c2.1.clone()), env));
    let (a1, b1, d1) = lin_coeffs(&e1, &k1, &k2);
    let (a2, b2, d2) = lin_coeffs(&e2, &k1, &k2);
    // a*k1 + b*k2 = d  (d = -constant term)
    let det = ratsimp(&Expr::sub(Expr::mul(a1.clone(), b2.clone()), Expr::mul(b1.clone(), a2.clone())));
    if det == Expr::int(0) { return None; }
    let k1v = ratsimp(&Expr::div(
        Expr::sub(Expr::mul(d1.clone(), b2.clone()), Expr::mul(b1, d2.clone())), det.clone()));
    let k2v = ratsimp(&Expr::div(
        Expr::sub(Expr::mul(a1, d2), Expr::mul(d1, a2)), det));
    let out = subst(&k1v, &k1, &subst(&k2v, &k2, expr));
    Some(meval(&out, env))
}

fn lin_coeffs(e: &Expr, k1: &Expr, k2: &Expr) -> (Expr, Expr, Expr) {
    let a = coeff_of(e, k1);
    let b = coeff_of(e, k2);
    let c = simplify(&subst(&Expr::int(0), k1, &subst(&Expr::int(0), k2, e)));
    // a*k1 + b*k2 + c = 0  ⇒  a*k1 + b*k2 = -c
    (a, b, simplify(&Expr::neg(c)))
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
