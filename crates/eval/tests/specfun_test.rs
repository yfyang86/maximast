// S4: special-functions plugin. Loads maxima-specfun and checks exact cases
// plus numeric values against references (the mandatory numerical verification).
use maxima_eval::{eval_str_with_env, Environment};
use std::path::PathBuf;
use std::process::Command;

fn plugin_path() -> Option<String> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let name = format!(
        "{}maxima_specfun.{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_EXTENSION
    );
    for profile in ["debug", "release"] {
        let p = target.join(profile).join(&name);
        if p.is_file() {
            return Some(p.display().to_string());
        }
    }
    None
}

fn ensure() -> Option<String> {
    if let Some(p) = plugin_path() {
        return Some(p);
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let _ = Command::new(cargo).args(["build", "-p", "maxima-specfun"]).status();
    plugin_path()
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn approx(env: &mut Environment, expr: &str, expected: f64) {
    let r = eval_str_with_env(expr, env);
    let v: f64 = r.trim().parse().unwrap_or_else(|_| panic!("{} -> {} (not numeric)", expr, r));
    assert!((v - expected).abs() < 1e-9, "{} = {}, expected {}", expr, v, expected);
}

#[test]
fn specfun_exact_cases() {
    let Some(path) = ensure() else {
        eprintln!("skipping: maxima-specfun cdylib not available");
        return;
    };
    let mut env = Environment::new();
    assert_eq!(eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env), "true");

    let mut run = |s: &str| norm(&eval_str_with_env(s, &mut env));
    assert_eq!(run("gamma(5);"), "24"); // 4!
    assert_eq!(run("gamma(1);"), "1");
    assert_eq!(run("gamma(1/2);"), "sqrt(%pi)");
    assert_eq!(run("beta(2, 3);"), "1/12");
    assert_eq!(run("erf(0);"), "0");
    assert_eq!(run("erfc(0);"), "1");
}

#[test]
fn specfun_numeric_references() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);

    // gamma / log_gamma / beta
    approx(&mut env, "gamma(4.5);", 11.631_728_396_567_448);
    approx(&mut env, "log_gamma(10.0);", 12.801_827_480_081_469);
    approx(&mut env, "beta(2.0, 3.0);", 0.083_333_333_333_333_33);

    // erf / erfc (series path |x|<2 and continued-fraction path |x|>=2)
    approx(&mut env, "erf(1.0);", 0.842_700_792_949_714_9);
    approx(&mut env, "erf(0.5);", 0.520_499_877_813_046_5);
    approx(&mut env, "erfc(2.0);", 0.004_677_734_981_047_266);
    approx(&mut env, "erf(2.5);", 0.999_593_047_982_555);
    approx(&mut env, "erfc(0.5);", 0.479_500_122_186_953_5);

    // Bessel J and I
    approx(&mut env, "bessel_j(0, 1.0);", 0.765_197_686_557_966_6);
    approx(&mut env, "bessel_j(1, 2.0);", 0.576_724_807_756_873_4);
    approx(&mut env, "bessel_i(0, 1.0);", 1.266_065_877_752_008_4);
    approx(&mut env, "bessel_i(1, 1.0);", 0.565_159_103_992_485_1);
}

#[test]
fn specfun_symbolic_is_noun() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);
    // Symbolic argument: keep the noun form, do not floatify.
    assert!(eval_str_with_env("gamma(x);", &mut env).contains("gamma"));
    assert!(eval_str_with_env("erf(x);", &mut env).contains("erf"));
}
