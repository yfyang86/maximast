use maxima_core::{Expr, Operator};

fn as_str(e: &Expr) -> Option<&str> {
    if let Expr::String(s) = e { Some(s) } else { None }
}

pub(crate) fn eval_string_func(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "slength" => {
            as_str(args.first()?).map(|s| Expr::int(s.len() as i64))
        }
        "charat" => {
            if args.len() == 2 {
                let s = as_str(&args[0])?;
                let n = if let Expr::Integer(i) = &args[1] { *i } else { return None };
                if n >= 1 && (n as usize) <= s.len() {
                    let ch = s.chars().nth((n - 1) as usize)?;
                    Some(Expr::String(ch.to_string().into()))
                } else { None }
            } else { None }
        }
        "substring" => {
            if args.len() >= 2 {
                let s = as_str(&args[0])?;
                let start = if let Expr::Integer(i) = &args[1] { *i } else { return None };
                if start < 1 { return None; }
                let start_idx = (start - 1) as usize;
                if start_idx > s.len() { return None; }
                let end_idx = if args.len() >= 3 {
                    if let Expr::Integer(e) = &args[2] { (*e as usize).min(s.len()) }
                    else { return None }
                } else {
                    s.len()
                };
                if end_idx < start_idx { return None; }
                // Maxima substring is 1-indexed, end is exclusive
                let result: String = s.chars().skip(start_idx).take(end_idx - start_idx).collect();
                Some(Expr::String(result.into()))
            } else { None }
        }
        "ssearch" => {
            if args.len() == 2 {
                let pattern = as_str(&args[0])?;
                let s = as_str(&args[1])?;
                match s.find(pattern) {
                    Some(pos) => {
                        let char_pos = s[..pos].chars().count() + 1;
                        Some(Expr::int(char_pos as i64))
                    }
                    None => Some(Expr::sym("false"))
                }
            } else { None }
        }
        "ssubst" => {
            // ssubst(new, old, s)
            if args.len() == 3 {
                let new_str = as_str(&args[0])?;
                let old_str = as_str(&args[1])?;
                let s = as_str(&args[2])?;
                Some(Expr::String(s.replace(old_str, new_str).into()))
            } else { None }
        }
        "strim" => {
            as_str(args.first()?).map(|s| Expr::String(s.trim().to_string().into()))
        }
        "striml" => {
            as_str(args.first()?).map(|s| Expr::String(s.trim_start().to_string().into()))
        }
        "strimr" => {
            as_str(args.first()?).map(|s| Expr::String(s.trim_end().to_string().into()))
        }
        "split" => {
            if let Some(s) = as_str(args.first()?) {
                let delim = if args.len() >= 2 {
                    as_str(&args[1]).unwrap_or(" ")
                } else { " " };
                let parts: Vec<Expr> = s.split(delim)
                    .filter(|p| !p.is_empty())
                    .map(|p| Expr::String(p.to_string().into()))
                    .collect();
                Some(Expr::list(parts))
            } else { None }
        }
        "supcase" => {
            as_str(args.first()?).map(|s| Expr::String(s.to_uppercase().into()))
        }
        "sdowncase" => {
            as_str(args.first()?).map(|s| Expr::String(s.to_lowercase().into()))
        }
        "sequal" => {
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (as_str(&args[0]), as_str(&args[1])) {
                    Some(if a == b { Expr::sym("true") } else { Expr::sym("false") })
                } else { None }
            } else { None }
        }
        "parse_string" => {
            if let Some(s) = as_str(args.first()?) {
                let input = if s.ends_with(';') || s.ends_with('$') {
                    s.to_string()
                } else {
                    format!("{};", s)
                };
                let exprs = maxima_parser::parse_multi(&input);
                exprs.into_iter().last().or(Some(Expr::sym("done")))
            } else { None }
        }
        "numberp" if args.len() == 1 => {
            // numberp for strings returns false — handled here to avoid fallthrough
            if matches!(&args[0], Expr::String(_)) {
                Some(Expr::sym("false"))
            } else { None }
        }
        _ => None,
    }
}
