use std::io::Read;
use std::time::Instant;

use maxima_eval::{Environment, eval_str_with_env, eval_expr_with_env};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

const VERSION: &str = "8.0.0";

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";

// ==================== Tab Completion ====================

const BUILTIN_FUNCTIONS: &[&str] = &[
    "abs", "acos", "append", "apply", "asin", "assume", "atan",
    "batch", "batchload", "binomial", "block",
    "ceiling", "charpoly", "coeff", "concat", "cos", "cosh", "cot", "coth", "csc", "csch",
    "declare", "determinant", "diff", "display",
    "eigenvalues", "eigenvectors", "endcons", "ev", "expand", "exp",
    "erf", "erfc", "erfi",
    "expintegral_ei", "expintegral_li", "expintegral_si", "expintegral_ci",
    "fresnel_s", "fresnel_c",
    "eliminate", "factor", "factor_multivariate", "facts", "file_search",
    "file_search_maxima", "first", "float",
    "floor", "forget", "fourth",
    "gcd", "groebner_basis",
    "ideal_contains", "ideal_intersect", "ideal_product", "ideal_sum",
    "integrate", "invert", "is",
    "kill",
    "last", "length", "limit", "linsolve", "load", "load_pathname", "loaded_files", "log",
    "makelist", "map", "matrix", "max", "min", "mod",
    "part", "partfrac", "polysys_solve", "primep", "print", "printfile", "product",
    "quit",
    "radcan", "ratsimp", "remainder", "require", "rest", "reverse", "round",
    "save", "sconcat", "sec", "sech", "second", "setup_autoload",
    "sin", "sinh", "solve", "sort", "sqrt",
    "stringout", "subst", "sum",
    "tan", "tanh", "taylor", "tex", "third", "transpose", "trigexpand", "trigsimp", "truncate",
];

const KEYWORDS: &[&str] = &[
    "and", "block", "do", "else", "elseif", "for", "from", "if",
    "in", "lambda", "not", "or", "return", "step", "then", "thru", "while",
];

const CONSTANTS: &[&str] = &[
    "%e", "%gamma", "%i", "%phi", "%pi",
    "false", "inf", "minf", "true",
];

struct MaximaHelper;

impl Completer for MaximaHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, prefix) = find_word_start(line, pos);
        if prefix.is_empty() {
            return Ok((pos, vec![]));
        }

        let mut candidates: Vec<Pair> = Vec::new();

        for &name in BUILTIN_FUNCTIONS {
            if name.starts_with(prefix) {
                candidates.push(Pair {
                    display: name.to_string(),
                    replacement: format!("{}(", name),
                });
            }
        }
        for &name in KEYWORDS {
            if name.starts_with(prefix) {
                candidates.push(Pair {
                    display: name.to_string(),
                    replacement: format!("{} ", name),
                });
            }
        }
        for &name in CONSTANTS {
            if name.starts_with(prefix) {
                candidates.push(Pair {
                    display: name.to_string(),
                    replacement: name.to_string(),
                });
            }
        }

        // If only one match, use it directly; if prefix already has '(' don't add another
        if candidates.len() == 1 {
            let next_char = line.get(pos..pos + 1).unwrap_or("");
            if next_char == "(" {
                candidates[0].replacement = candidates[0].display.clone();
            }
        }

        Ok((start, candidates))
    }
}

fn find_word_start(line: &str, pos: usize) -> (usize, &str) {
    let before = &line[..pos];
    let start = before
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '%')
        .map(|i| i + 1)
        .unwrap_or(0);
    (start, &line[start..pos])
}

impl Hinter for MaximaHelper {
    type Hint = String;
}

impl Highlighter for MaximaHelper {}
impl Validator for MaximaHelper {}
impl Helper for MaximaHelper {}

// ==================== CLI Argument Parsing ====================

enum Mode {
    Repl { quiet: bool },
    Batch { file: String, quiet: bool },
    Eval { expr: String },
    Stdin,
    Help,
    Version,
}

fn parse_args(args: &[String]) -> Mode {
    let mut i = 1;
    let mut quiet = false;
    let mut very_quiet = false;
    let mut batch_file = None;
    let mut eval_expr = None;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Mode::Help,
            "-v" | "--version" => return Mode::Version,
            "-q" | "--quiet" => { quiet = true; i += 1; }
            "--very-quiet" => { quiet = true; very_quiet = true; i += 1; }
            "-e" | "--eval" => {
                if i + 1 < args.len() {
                    eval_expr = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: -e requires an expression argument");
                    std::process::exit(1);
                }
            }
            "-b" | "--batch" => {
                if i + 1 < args.len() {
                    batch_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --batch requires a filename argument");
                    std::process::exit(1);
                }
            }
            arg if !arg.starts_with('-') => {
                batch_file = Some(arg.to_string());
                i += 1;
            }
            other => {
                eprintln!("Unknown option: {}", other);
                std::process::exit(1);
            }
        }
    }

    if let Some(expr) = eval_expr {
        return Mode::Eval { expr };
    }
    if let Some(file) = batch_file {
        return Mode::Batch { file, quiet: very_quiet };
    }
    if !atty_stdin() {
        return Mode::Stdin;
    }
    Mode::Repl { quiet }
}

fn print_help() {
    println!("Maxima Kernel (Rust) v{}", VERSION);
    println!();
    println!("USAGE:");
    println!("    maxima-kernel                     Start interactive REPL");
    println!("    maxima-kernel <file.mac>          Run script file");
    println!("    maxima-kernel -e \"expr;\"          Evaluate expression");
    println!("    echo \"expr;\" | maxima-kernel      Read from stdin");
    println!();
    println!("OPTIONS:");
    println!("    -e, --eval <expr>     Evaluate expression and exit");
    println!("    -b, --batch <file>    Run file in batch mode (no prompts)");
    println!("    -q, --quiet           Suppress banner");
    println!("    --very-quiet          Suppress banner and prompts");
    println!("    -v, --version         Print version");
    println!("    -h, --help            Print this help");
    println!();
    println!("ENVIRONMENT:");
    println!("    NO_COLOR=1            Disable syntax highlighting");
    println!();
    println!("EXAMPLES:");
    println!("    maxima-kernel -e \"factor(x^6-1);\"");
    println!("    maxima-kernel -e \"integrate(sin(x)^2, x);\"");
    println!("    maxima-kernel test.mac");
    println!("    echo 'diff(sin(x),x);' | maxima-kernel");
}

// ==================== Execution Modes ====================

fn run_eval(expr: &str) -> i32 {
    let mut env = Environment::new();
    let input = if expr.ends_with(';') || expr.ends_with('$') {
        expr.to_string()
    } else {
        format!("{};", expr)
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eval_str_with_env(&input, &mut env)
    })) {
        Ok(result) => { println!("{}", result); 0 }
        Err(_) => { eprintln!("Error: evaluation failed"); 1 }
    }
}

fn run_batch(file: &str, quiet: bool) -> i32 {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: cannot read '{}': {}", file, e);
            return 1;
        }
    };
    run_script(&content, quiet)
}

fn run_stdin() -> i32 {
    let mut content = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut content) {
        eprintln!("Error reading stdin: {}", e);
        return 1;
    }
    run_script(&content, false)
}

fn run_script(content: &str, quiet: bool) -> i32 {
    let mut env = Environment::new();
    let stmts = maxima_parser::parse_multi_with_display(content);

    for (expr, display) in stmts {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eval_expr_with_env(&expr, &mut env)
        })) {
            Ok(output) => {
                if !quiet && display {
                    println!("{}", output);
                }
            }
            Err(_) => {
                eprintln!("Error: evaluation failed for: {}", expr);
                return 1;
            }
        }
    }
    0
}

// ==================== REPL ====================

fn print_banner(color: bool) {
    if color {
        println!("{BOLD}{BLUE}╔══════════════════════════════════════════════════╗{RESET}");
        println!("{BOLD}{BLUE}║{RESET}  {BOLD}Maxima Kernel{RESET} {DIM}(Rust){RESET}  {YELLOW}v{VERSION}{RESET}  {BOLD}    /\\_/\\ {RESET}   ╔══╗ {BOLD}{BLUE}║{RESET}");
        println!("{BOLD}{BLUE}║{RESET}  {DIM}A Computer Algebra System{RESET}        {BOLD}( o.o ) {RESET}  ║⊙⊙║ {BOLD}{BLUE}║{RESET}");
        println!("{BOLD}{BLUE}║{RESET}  {DIM}MIT / Apache-2.0{RESET}                {BOLD}  > ^ < {RESET}   ╚══╝ {BOLD}{BLUE}║{RESET}");
        println!("{BOLD}{BLUE}║{RESET}  {DIM}Author: Yifan Yang{RESET}              {BOLD} /     \\{RESET}   ╲__╱ {BOLD}{BLUE}║{RESET}");
        println!("{BOLD}{BLUE}╚══════════════════════════════════════════════════╝{RESET}");
        println!();
        println!("  {DIM}Type{RESET} {BOLD}quit;{RESET} {DIM}to exit.  End expressions with{RESET} {BOLD};{RESET} {DIM}or{RESET} {BOLD}${RESET}");
        println!("  {DIM}Use ↑/↓ for history, Tab for completion{RESET}");
        println!();
    } else {
        println!("Maxima Kernel (Rust) v{}", VERSION);
        println!("Licensed under MIT / Apache-2.0. Type 'quit;' to exit.\n");
    }
}

fn input_prompt(label: usize, color: bool) -> String {
    if color { format!("{BOLD}{GREEN}(%i{label}){RESET} ") }
    else { format!("(%i{}) ", label) }
}

fn continuation_prompt(color: bool) -> String {
    if color { format!("{DIM}  ...  {RESET}") }
    else { "       ".to_string() }
}

fn has_terminator(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() { return false; }
    let mut in_string = false;
    let mut in_comment = 0;
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if in_string {
            if chars[i] == '\\' && i + 1 < chars.len() { i += 2; continue; }
            if chars[i] == '"' { in_string = false; }
        } else if in_comment > 0 {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' { in_comment -= 1; i += 2; continue; }
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' { in_comment += 1; i += 2; continue; }
        } else {
            if chars[i] == '"' { in_string = true; }
            else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' { in_comment += 1; i += 2; continue; }
            else if chars[i] == ';' || chars[i] == '$' { return true; }
        }
        i += 1;
    }
    false
}

fn highlight_output(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '0'..='9' => {
                result.push_str(CYAN); result.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' || next == '/' { result.push(chars.next().unwrap()); }
                    else { break; }
                }
                result.push_str(RESET);
            }
            '+' | '*' | '^' | '=' | '#' | '<' | '>' | '-' => {
                result.push_str(YELLOW); result.push(ch);
                if let Some(&next) = chars.peek() {
                    if (ch == '<' || ch == '>' || ch == ':') && next == '=' { result.push(chars.next().unwrap()); }
                }
                result.push_str(RESET);
            }
            '(' | ')' | '[' | ']' => { result.push_str(BOLD); result.push(ch); result.push_str(RESET); }
            '"' => {
                result.push_str(GREEN); result.push(ch);
                while let Some(next) = chars.next() {
                    result.push(next);
                    if next == '"' { break; }
                    if next == '\\' { if let Some(esc) = chars.next() { result.push(esc); } }
                }
                result.push_str(RESET);
            }
            _ if ch.is_alphabetic() || ch == '%' || ch == '_' => {
                let mut word = String::new(); word.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '_' || next == '%' { word.push(chars.next().unwrap()); }
                    else { break; }
                }
                let color = match word.as_str() {
                    "true" | "false" | "done" | "und" | "ind" => MAGENTA,
                    "inf" | "minf" | "infinity" => RED,
                    "%pi" | "%e" | "%i" | "%phi" => BOLD,
                    "sin" | "cos" | "tan" | "exp" | "log" | "sqrt"
                    | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh"
                    | "diff" | "integrate" | "limit" | "taylor"
                    | "factor" | "expand" | "ratsimp" | "solve" | "matrix"
                    | "abs" | "gcd" | "sum" | "product" | "binomial"
                    | "determinant" | "invert" | "eigenvalues" | "eigenvectors"
                    | "assume" | "forget" | "is" | "declare"
                    | "tex" | "batch" | "load" | "save" | "quit" => BLUE,
                    "lambda" | "block" | "if" | "then" | "else" | "for"
                    | "while" | "do" | "thru" | "in" | "return"
                    | "and" | "or" | "not" => YELLOW,
                    _ => "",
                };
                if !color.is_empty() { result.push_str(color); result.push_str(&word); result.push_str(RESET); }
                else { result.push_str(&word); }
            }
            _ => result.push(ch),
        }
    }
    result
}

fn run_repl(quiet: bool) -> i32 {
    let use_color = std::env::var("NO_COLOR").is_err() && atty_stdout();
    if !quiet { print_banner(use_color); }

    let mut rl = match Editor::new() {
        Ok(r) => r,
        Err(e) => { eprintln!("Error initializing editor: {}", e); return 1; }
    };
    rl.set_helper(Some(MaximaHelper));
    let mut env = Environment::new();
    let mut input_buf = String::new();
    let mut current_label = 0usize;

    loop {
        let prompt = if input_buf.is_empty() {
            current_label = env.next_label();
            input_prompt(current_label, use_color)
        } else {
            continuation_prompt(use_color)
        };

        match rl.readline(&prompt) {
            Ok(line) => {
                if !input_buf.is_empty() { input_buf.push('\n'); }
                input_buf.push_str(&line);
                let trimmed = input_buf.trim();
                if trimmed.is_empty() { input_buf.clear(); continue; }
                if !has_terminator(trimmed) { continue; }

                let _ = rl.add_history_entry(input_buf.trim());
                let suppress = trimmed.ends_with('$');

                if trimmed == "quit;" || trimmed == "quit();" { break; }

                let input = input_buf.clone();
                input_buf.clear();
                let start = Instant::now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    eval_str_with_env(&input, &mut env)
                }));
                let elapsed = start.elapsed();

                match result {
                    Ok(output) => {
                        if !suppress {
                            if use_color {
                                println!("{BOLD}{RED}(%o{current_label}){RESET} {}", highlight_output(&output));
                            } else {
                                println!("(%o{}) {}", current_label, output);
                            }
                        }
                        if elapsed.as_millis() > 100 && use_color {
                            println!("{DIM}  [{:.3}s]{RESET}", elapsed.as_secs_f64());
                        }
                    }
                    Err(_) => {
                        if use_color { println!("{RED}{BOLD}Error:{RESET} {RED}evaluation failed{RESET}"); }
                        else { println!("Error: evaluation failed"); }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => { input_buf.clear(); continue; }
            Err(ReadlineError::Eof) => { break; }
            Err(err) => { eprintln!("Error: {:?}", err); return 1; }
        }
    }
    0
}

// ==================== Entry Point ====================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = parse_args(&args);

    let exit_code = match mode {
        Mode::Help => { print_help(); 0 }
        Mode::Version => { println!("maxima-kernel v{}", VERSION); 0 }
        Mode::Eval { expr } => run_eval(&expr),
        Mode::Batch { file, quiet } => run_batch(&file, quiet),
        Mode::Stdin => run_stdin(),
        Mode::Repl { quiet } => run_repl(quiet),
    };

    std::process::exit(exit_code);
}

fn atty_stdout() -> bool { unsafe { libc_isatty(1) != 0 } }
fn atty_stdin() -> bool { unsafe { libc_isatty(0) != 0 } }

extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}
