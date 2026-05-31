use maxima_eval::{Environment, eval_str_with_env};

struct RtestResult {
    total: usize,
    passed: usize,
    failed: usize,
    errors: usize,
    failures: Vec<(usize, String, String, String)>,
}

fn parse_rtest_pairs(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut lines = Vec::new();

    // Strip comments and join lines
    let mut in_comment = false;
    let mut cleaned = String::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !in_comment && i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            in_comment = true;
            i += 2;
            continue;
        }
        if in_comment && i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
            in_comment = false;
            i += 2;
            continue;
        }
        if !in_comment {
            cleaned.push(chars[i]);
        }
        i += 1;
    }

    // Split by ; and $, preserving which terminator
    let mut current = String::new();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut ci = 0;
    while ci < chars.len() {
        let ch = chars[ci];
        if ch == '"' {
            current.push(ch);
            ci += 1;
            while ci < chars.len() && chars[ci] != '"' {
                if chars[ci] == '\\' && ci + 1 < chars.len() {
                    current.push(chars[ci]);
                    ci += 1;
                }
                current.push(chars[ci]);
                ci += 1;
            }
            if ci < chars.len() {
                current.push(chars[ci]);
                ci += 1;
            }
            continue;
        }
        if ch == ';' || ch == '$' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                lines.push(trimmed);
            }
            current.clear();
        } else {
            current.push(ch);
        }
        ci += 1;
    }

    // Pair up: input, expected_output
    let mut li = 0;
    while li + 1 < lines.len() {
        pairs.push((lines[li].clone(), lines[li + 1].clone()));
        li += 2;
    }
    pairs
}

fn run_rtest(path: &str) -> RtestResult {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read {}: {}", path, e);
            return RtestResult {
                total: 0, passed: 0, failed: 0, errors: 1,
                failures: vec![],
            };
        }
    };

    let pairs = parse_rtest_pairs(&content);
    let mut env = Environment::new();
    let mut result = RtestResult {
        total: pairs.len(),
        passed: 0,
        failed: 0,
        errors: 0,
        failures: vec![],
    };

    for (i, (input, expected)) in pairs.iter().enumerate() {
        let actual = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eval_str_with_env(&format!("{};", input), &mut env)
        }));

        match actual {
            Ok(actual_str) => {
                let expected_str = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    eval_str_with_env(&format!("{};", expected), &mut env)
                }));
                match expected_str {
                    Ok(exp_str) => {
                        if actual_str == exp_str {
                            result.passed += 1;
                        } else {
                            result.failed += 1;
                            if result.failures.len() < 20 {
                                result.failures.push((
                                    i + 1,
                                    input.clone(),
                                    exp_str,
                                    actual_str,
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        result.errors += 1;
                    }
                }
            }
            Err(_) => {
                result.errors += 1;
            }
        }
    }

    result
}

#[test]
fn rtest1_progress() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/rtest1.mac");
    let result = run_rtest(path);

    println!("\n=== rtest1.mac Results ===");
    println!(
        "Total: {} | Passed: {} | Failed: {} | Errors: {}",
        result.total, result.passed, result.failed, result.errors
    );
    println!(
        "Pass rate: {:.1}%",
        if result.total > 0 {
            result.passed as f64 / result.total as f64 * 100.0
        } else {
            0.0
        }
    );

    if !result.failures.is_empty() {
        println!("\nFirst failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }

    // Track progress — we expect to pass at least some tests
    assert!(
        result.passed > 0,
        "Should pass at least some rtest1 pairs"
    );

    println!(
        "\nRC1 Progress: {}/{} rtest1 pairs passing",
        result.passed, result.total
    );
}

#[test]
fn rtest_basic_arithmetic() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/rtest1.mac");

    // Just verify the runner parses the file correctly
    let content = std::fs::read_to_string(path).unwrap();
    let pairs = parse_rtest_pairs(&content);
    assert!(pairs.len() > 10, "Should parse many test pairs, got {}", pairs.len());
}

/// Run a single rtest file and return (passed, total)
fn run_rtest_quick(name: &str) -> (usize, usize) {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), name);
    let result = run_rtest(&path);
    (result.passed, result.total)
}

#[test]
fn rtest_abs() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_abs.mac");
    let result = run_rtest(&path);
    println!("rtest_abs: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
    assert!(result.passed > 0, "should pass some rtest_abs tests");
}

#[test]
fn rtest_boolean() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_boolean.mac");
    let result = run_rtest(&path);
    println!("rtest_boolean: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
    assert!(result.passed > 0);
}

#[test]
fn rtest_equal() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_equal.mac");
    let result = run_rtest(&path);
    println!("rtest_equal: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
    assert!(result.passed > 0);
}

#[test]
fn rtest_algebraic() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_algebraic.mac");
    let result = run_rtest(&path);
    println!("rtest_algebraic: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
    assert!(result.passed > 0);
}

#[test]
fn rtest_gcd() {
    let (passed, total) = run_rtest_quick("rtest_gcd.mac");
    println!("rtest_gcd: {}/{} ({:.0}%)", passed, total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 0.0 });
}

#[test]
fn rtest_everysome() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_everysome.mac");
    let result = run_rtest(&path);
    println!("rtest_everysome: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
}

#[test]
fn rtest_diff_invtrig() {
    let (passed, total) = run_rtest_quick("rtest_diff_invtrig.mac");
    println!("rtest_diff_invtrig: {}/{} ({:.0}%)", passed, total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 0.0 });
}

#[test]
fn rtest_dot() {
    let path = format!("{}/{}", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests"), "rtest_dot.mac");
    let result = run_rtest(&path);
    println!("rtest_dot: {}/{} ({:.0}%)", result.passed, result.total,
        if result.total > 0 { result.passed as f64 / result.total as f64 * 100.0 } else { 0.0 });
    if !result.failures.is_empty() {
        println!("First failures:");
        for (num, input, expected, actual) in &result.failures {
            println!("  #{}: {} => expected '{}', got '{}'", num, input, expected, actual);
        }
    }
}

#[test]
fn rtest_ask1() {
    let (passed, total) = run_rtest_quick("rtest_ask1.mac");
    println!("rtest_ask1: {}/{} ({:.0}%)", passed, total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 0.0 });
}

#[test]
fn rtest_carg() {
    let (passed, total) = run_rtest_quick("rtest_carg.mac");
    println!("rtest_carg: {}/{} ({:.0}%)", passed, total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 0.0 });
}

#[test]
#[ignore] // Slow: scans all 99 rtest files. Run with: cargo test -- --ignored
fn rtest_multi_file_scan() {
    let test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests");
    let mut files: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("rtest") && name.ends_with(".mac") {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    files.sort();
    let mut total_pass = 0;
    let mut total_tests = 0;
    let mut passing_files = 0;

    println!("\n=== Multi-file rtest scan ===");
    for file in &files {
        let result = run_rtest(file);
        let name = file.rsplit('/').next().unwrap_or(file);
        total_pass += result.passed;
        total_tests += result.total;
        let pct = if result.total > 0 {
            result.passed as f64 / result.total as f64 * 100.0
        } else {
            0.0
        };
        if pct >= 30.0 {
            let marker = if pct >= 50.0 { "***" } else { "" };
            println!("  {} : {}/{} ({:.0}%) {}", name, result.passed, result.total, pct, marker);
            if pct >= 50.0 { passing_files += 1; }
        }
    }
    println!("\nTotal: {}/{} pairs across {} files", total_pass, total_tests, files.len());
    println!("Files with >=50% pass rate: {}", passing_files);
}
