use maxima_core::{Expr, Operator};
use crate::helpers::{subst, to_f64};
use crate::eval::meval;
use crate::env::Environment;

pub(crate) fn eval_plot(name: &str, args: &[Expr], env: &mut Environment) -> Option<Expr> {
    match name {
        "plot2d" => {
            if args.len() >= 2 {
                plot2d(&args[0], &args[1..], env)
            } else { None }
        }
        "gnuplot_script" => {
            if args.len() >= 2 {
                gnuplot_script(&args[0], &args[1..], env)
            } else { None }
        }
        _ => None,
    }
}

fn plot2d(expr_or_list: &Expr, rest: &[Expr], env: &mut Environment) -> Option<Expr> {
    // plot2d(expr, [var, lo, hi])  or  plot2d([expr1, expr2], [var, lo, hi])
    let (exprs, var, lo, hi) = parse_plot_args(expr_or_list, rest)?;

    let n_points = 500;
    let dx = (hi - lo) / (n_points as f64);

    let mut all_series: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    for expr in &exprs {
        let mut points = Vec::new();
        for i in 0..=n_points {
            let x = lo + dx * (i as f64);
            let val = eval_at_point(expr, &var, x, env);
            if let Some(y) = val {
                if y.is_finite() { points.push((x, y)); }
            }
        }
        all_series.push(points);
        labels.push(expr.to_string());
    }

    // Generate SVG via plotters
    let filename = "maxima_plot.svg";
    if render_svg(filename, &all_series, &labels, lo, hi).is_ok() {
        println!("Plot saved to {}", filename);
        Some(Expr::String(filename.to_string().into()))
    } else {
        None
    }
}

fn gnuplot_script(expr_or_list: &Expr, rest: &[Expr], env: &mut Environment) -> Option<Expr> {
    let (exprs, var, lo, hi) = parse_plot_args(expr_or_list, rest)?;
    let n_points = 500;
    let dx = (hi - lo) / (n_points as f64);

    let mut script = String::new();
    script.push_str("set terminal svg size 800,600\n");
    script.push_str("set output 'maxima_plot.svg'\n");
    script.push_str(&format!("set xrange [{}:{}]\n", lo, hi));
    script.push_str("set grid\n");

    let mut data_blocks = Vec::new();
    for (_idx, expr) in exprs.iter().enumerate() {
        let mut block = String::new();
        for i in 0..=n_points {
            let x = lo + dx * (i as f64);
            if let Some(y) = eval_at_point(expr, &var, x, env) {
                if y.is_finite() {
                    block.push_str(&format!("{} {}\n", x, y));
                }
            }
        }
        block.push_str("e\n");
        data_blocks.push((expr.to_string(), block));
    }

    script.push_str("plot ");
    for (i, (label, _)) in data_blocks.iter().enumerate() {
        if i > 0 { script.push_str(", "); }
        script.push_str(&format!("'-' title '{}' with lines", label));
    }
    script.push('\n');
    for (_, block) in &data_blocks {
        script.push_str(block);
    }

    let filename = "maxima_plot.gnuplot";
    std::fs::write(filename, &script).ok()?;
    println!("Gnuplot script saved to {}", filename);
    Some(Expr::String(filename.to_string().into()))
}

fn parse_plot_args(expr_or_list: &Expr, rest: &[Expr]) -> Option<(Vec<Expr>, Expr, f64, f64)> {
    // Parse [var, lo, hi] from rest
    let range = rest.first()?;
    let (var, lo, hi) = if let Expr::List { op: Operator::MList, args, .. } = range {
        if args.len() >= 3 {
            let lo_expr = crate::helpers::expr_to_float(&args[1]);
            let hi_expr = crate::helpers::expr_to_float(&args[2]);
            (args[0].clone(), to_f64(&lo_expr)?, to_f64(&hi_expr)?)
        } else { return None; }
    } else { return None; };

    let exprs = match expr_or_list {
        Expr::List { op: Operator::MList, args, .. } => args.clone(),
        _ => vec![expr_or_list.clone()],
    };

    Some((exprs, var, lo, hi))
}

fn eval_at_point(expr: &Expr, var: &Expr, x: f64, env: &mut Environment) -> Option<f64> {
    let substituted = subst(&Expr::Float(x), var, expr);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        meval(&substituted, env)
    })).ok()?;
    to_f64(&result)
}

fn render_svg(
    filename: &str,
    series: &[Vec<(f64, f64)>],
    labels: &[String],
    x_lo: f64, x_hi: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;

    let mut y_lo = f64::MAX;
    let mut y_hi = f64::MIN;
    for pts in series {
        for (_, y) in pts {
            if *y < y_lo { y_lo = *y; }
            if *y > y_hi { y_hi = *y; }
        }
    }
    if y_lo >= y_hi { y_lo = -1.0; y_hi = 1.0; }
    let margin = (y_hi - y_lo) * 0.05;
    y_lo -= margin;
    y_hi += margin;

    let root = SVGBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .x_label_area_size(30)
        .y_label_area_size(50)
        .build_cartesian_2d(x_lo..x_hi, y_lo..y_hi)?;

    chart.configure_mesh().draw()?;

    let colors = [&RED, &BLUE, &GREEN, &MAGENTA, &CYAN];
    for (i, (pts, label)) in series.iter().zip(labels.iter()).enumerate() {
        let color = colors[i % colors.len()];
        chart.draw_series(LineSeries::new(
            pts.iter().copied(),
            color.stroke_width(2),
        ))?.label(label.as_str()).legend(move |(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(2))
        });
    }

    if series.len() > 1 {
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .draw()?;
    }

    root.present()?;
    Ok(())
}
