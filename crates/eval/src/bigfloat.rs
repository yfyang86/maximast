use maxima_core::{BigFloatVal, Expr, Operator, resolve, intern};
use astro_float::{BigFloat, Consts, RoundingMode, Radix};

const RM: RoundingMode = RoundingMode::ToEven;

pub(crate) fn eval_bfloat_func(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "bfloat" => {
            let digits = get_fpprec_decimal(env).max(1);
            let bits = decimal_to_bits(digits);
            let arg = args.first()?;
            // A rootof noun is refined numerically rather than evaluated termwise.
            if let Expr::List { op: Operator::Named(id), args: ra, .. } = arg {
                if resolve(*id) == "rootof" {
                    return crate::rootof::eval_rootof_bfloat(ra, bits, digits);
                }
            }
            let mut cc = Consts::new().ok()?;
            let v = to_bigfloat(arg, bits, &mut cc)?;
            Some(wrap(&v, bits, digits, &mut cc))
        }
        "fpprec" => {
            if args.is_empty() {
                return Some(Expr::int(get_fpprec_decimal(env)));
            }
            if let Some(Expr::Integer(n)) = args.first() {
                set_fpprec(env, *n);
                return Some(Expr::int(*n));
            }
            None
        }
        _ => None,
    }
}

/// Bigfloat "contagion": fold an arithmetic node whose operands are all numbers
/// and at least one is a bigfloat, computing at the widest operand precision.
/// Returns None if any operand is non-numeric (leaving the node to the normal
/// simplifier) so this never changes a non-bigfloat result.
pub(crate) fn fold_numeric(op: &Operator, args: &[Expr]) -> Option<Expr> {
    let mut max_bits = 0u32;
    let mut has_big = false;
    for a in args {
        match a {
            Expr::Integer(_) | Expr::BigInt(_) | Expr::Rational { .. } | Expr::Float(_) => {}
            Expr::BigFloat(b) => { has_big = true; max_bits = max_bits.max(b.bits); }
            _ => return None,
        }
    }
    if !has_big { return None; }
    let bits = (max_bits.max(64)) as usize;
    let digits = (((bits as f64 - 8.0) / std::f64::consts::LOG2_10).floor() as i64).max(1);
    let expr = Expr::List { op: op.clone(), simplified: false, args: args.to_vec() };
    let mut cc = Consts::new().ok()?;
    let v = to_bigfloat(&expr, bits, &mut cc)?;
    Some(wrap(&v, bits, digits, &mut cc))
}

fn get_fpprec_decimal(env: &crate::env::Environment) -> i64 {
    let id = intern("fpprec");
    if let Some(Expr::Integer(n)) = env.get(id) { *n } else { 16 }
}

fn set_fpprec(env: &mut crate::env::Environment, digits: i64) {
    let id = intern("fpprec");
    env.set(id, Expr::int(digits));
}

/// Working precision in bits for `digits` decimal places, plus a few guard bits
/// so the displayed digits are all correct after rounding. Floored at one
/// machine word (64 bits) — astro-float yields NaN below that.
fn decimal_to_bits(digits: i64) -> usize {
    (((digits as f64) * std::f64::consts::LOG2_10).ceil() as usize + 8).max(64)
}

/// Wrap an astro-float value as an `Expr::BigFloat`, formatted to `digits`
/// significant decimal places.
fn wrap(v: &BigFloat, bits: usize, digits: i64, cc: &mut Consts) -> Expr {
    let s = v.format(Radix::Dec, RM, cc).unwrap_or_default();
    let s = round_sig(&s, digits as usize);
    Expr::BigFloat(Box::new(BigFloatVal { digits: s.into_boxed_str(), bits: bits as u32 }))
}

/// Truncate a backend decimal string `"d.ddddd...e±N"` to `n` significant
/// digits, keeping the exponent and a normalised `d.ddd` mantissa (astro-float
/// always emits one digit before the point). Trims the guard digits so the
/// display matches the requested fpprec; rounding is left to the backend.
pub(crate) fn round_sig_pub(s: &str, n: usize) -> String { round_sig(s, n) }

fn round_sig(s: &str, n: usize) -> String {
    let (mantissa, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (s, None),
    };
    let neg = mantissa.starts_with('-');
    let core: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let kept: String = core.chars().take(n.max(1)).collect();
    let mut out = String::new();
    if neg { out.push('-'); }
    if kept.is_empty() {
        out.push_str("0.0");
    } else {
        out.push_str(&kept[..1]);
        out.push('.');
        if kept.len() > 1 { out.push_str(&kept[1..]); } else { out.push('0'); }
    }
    if let Some(e) = exp { out.push('e'); out.push_str(e); }
    out
}

/// Recursively evaluate a closed-form expression to an astro-float value at
/// `bits` precision. Returns None for anything non-numeric (free symbol,
/// unsupported function) so `bfloat` leaves it unevaluated.
fn to_bigfloat(expr: &Expr, bits: usize, cc: &mut Consts) -> Option<BigFloat> {
    let f = |x: f64| BigFloat::from_f64(x, bits);
    match expr {
        Expr::Integer(n) => Some(BigFloat::from_i64(*n, bits)),
        Expr::Rational { num, den } =>
            Some(BigFloat::from_i64(*num, bits).div(&BigFloat::from_i64(*den, bits), bits, RM)),
        Expr::Float(x) => Some(f(*x)),
        Expr::BigFloat(b) => Some(BigFloat::parse(&b.digits, Radix::Dec, bits, RM, cc)),
        Expr::BigInt(b) => Some(BigFloat::parse(&b.to_string(), Radix::Dec, bits, RM, cc)),
        Expr::Symbol(id) => match resolve(*id).as_str() {
            "%pi" => Some(cc.pi(bits, RM)),
            "%e" => Some(cc.e(bits, RM)),
            "%gamma" => Some(BigFloat::parse(GAMMA, Radix::Dec, bits, RM, cc)),
            "%phi" => {
                let five = BigFloat::from_i64(5, bits).sqrt(bits, RM);
                Some(five.add(&BigFloat::from_i64(1, bits), bits, RM)
                    .div(&BigFloat::from_i64(2, bits), bits, RM))
            }
            _ => None,
        },
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut acc = BigFloat::from_i64(0, bits);
            for a in args { acc = acc.add(&to_bigfloat(a, bits, cc)?, bits, RM); }
            Some(acc)
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut acc = BigFloat::from_i64(1, bits);
            for a in args { acc = acc.mul(&to_bigfloat(a, bits, cc)?, bits, RM); }
            Some(acc)
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let base = to_bigfloat(&args[0], bits, cc)?;
            // Integer exponents are exact and keep sign branches simple.
            if let Expr::Integer(n) = &args[1] {
                return Some(base.powi(n.unsigned_abs() as usize, bits, RM).maybe_recip(*n, bits));
            }
            let exp = to_bigfloat(&args[1], bits, cc)?;
            Some(base.pow(&exp, bits, RM, cc))
        }
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let x = to_bigfloat(&args[0], bits, cc)?;
            let r = match resolve(*id).as_str() {
                "sqrt" => x.sqrt(bits, RM),
                "exp" => x.exp(bits, RM, cc),
                "log" | "ln" => x.ln(bits, RM, cc),
                "sin" => x.sin(bits, RM, cc),
                "cos" => x.cos(bits, RM, cc),
                "tan" => x.tan(bits, RM, cc),
                "asin" => x.asin(bits, RM, cc),
                "acos" => x.acos(bits, RM, cc),
                "atan" => x.atan(bits, RM, cc),
                "sinh" => x.sinh(bits, RM, cc),
                "cosh" => x.cosh(bits, RM, cc),
                "tanh" => x.tanh(bits, RM, cc),
                "abs" => x.abs(),
                _ => return None,
            };
            Some(r)
        }
        _ => None,
    }
}

/// Euler–Mascheroni constant to 60 digits (astro-float has no built-in).
const GAMMA: &str = "0.577215664901532860606512090082402431042159335939923598805767";

trait PowiRecip {
    fn maybe_recip(self, n: i64, bits: usize) -> BigFloat;
}
impl PowiRecip for BigFloat {
    fn maybe_recip(self, n: i64, bits: usize) -> BigFloat {
        if n < 0 { BigFloat::from_i64(1, bits).div(&self, bits, RM) } else { self }
    }
}
