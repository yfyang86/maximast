//! Built-in `help(...)` documentation system.
//!
//! Help data is stored in `src/help.toml` and embedded into the binary at
//! compile time. Each entry describes one function, its aliases, usage,
//! arguments, return value, and optional references/authors.

use maxima_core::{Expr, resolve};
use std::collections::HashMap;
use std::sync::OnceLock;

const HELP_TOML: &str = include_str!("help.toml");

#[derive(Debug, Clone)]
pub struct FunctionHelp {
    pub name: String,
    pub alias: Vec<String>,
    pub title: String,
    pub description: String,
    pub usage: String,
    pub arguments: String,
    pub details: String,
    pub value: String,
    pub references: Vec<String>,
    pub authors: Vec<String>,
}

fn as_string(value: &toml::Value) -> String {
    value.as_str().unwrap_or("").to_string()
}

fn as_string_list(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn load_help() -> HashMap<String, FunctionHelp> {
    let mut map = HashMap::new();
    let Ok(parsed) = HELP_TOML.parse::<toml::Value>() else {
        return map;
    };
    let Some(functions) = parsed.get("function").and_then(|v| v.as_array()) else {
        return map;
    };

    for entry in functions {
        let name = entry.get("name").map(as_string).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let alias = as_string_list(entry.get("alias"));
        let help = FunctionHelp {
            name: name.clone(),
            alias: alias.clone(),
            title: entry.get("title").map(as_string).unwrap_or_default(),
            description: entry.get("description").map(as_string).unwrap_or_default(),
            usage: entry.get("usage").map(as_string).unwrap_or_default(),
            arguments: entry.get("arguments").map(as_string).unwrap_or_default(),
            details: entry.get("details").map(as_string).unwrap_or_default(),
            value: entry.get("value").map(as_string).unwrap_or_default(),
            references: as_string_list(entry.get("references")),
            authors: as_string_list(entry.get("authors")),
        };
        map.insert(name.clone(), help.clone());
        for a in alias {
            map.insert(a, help.clone());
        }
    }
    map
}

fn help_map() -> &'static HashMap<String, FunctionHelp> {
    static MAP: OnceLock<HashMap<String, FunctionHelp>> = OnceLock::new();
    MAP.get_or_init(load_help)
}

fn section(name: &str, body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("{}:\n{}", name, trimmed))
    }
}

fn render_help(key: &str, help: &FunctionHelp, section_name: Option<&str>) -> String {
    if let Some(sec) = section_name {
        let value = match sec {
            "title" => help.title.trim(),
            "description" => help.description.trim(),
            "usage" => help.usage.trim(),
            "arguments" => help.arguments.trim(),
            "details" => help.details.trim(),
            "value" => help.value.trim(),
            "references" => return help.references.join("\n"),
            "authors" => return help.authors.join("\n"),
            _ => return format!("help: unknown section '{}' for '{}'", sec, key),
        };
        return value.to_string();
    }

    let mut parts = Vec::new();
    if help.title.is_empty() {
        parts.push(format!("== {} ==", help.name));
    } else {
        parts.push(format!("== {} ==\n{}", help.name, help.title.trim()));
    }

    if let Some(s) = section("DESCRIPTION", &help.description) { parts.push(s); }
    if let Some(s) = section("USAGE", &help.usage) { parts.push(s); }
    if let Some(s) = section("ARGUMENTS", &help.arguments) { parts.push(s); }
    if let Some(s) = section("DETAILS", &help.details) { parts.push(s); }
    if let Some(s) = section("VALUE", &help.value) { parts.push(s); }
    if !help.references.is_empty() {
        parts.push(format!("REFERENCES:\n{}", help.references.join("\n")));
    }
    if !help.authors.is_empty() {
        parts.push(format!("AUTHORS:\n{}", help.authors.join("\n")));
    }
    if !help.alias.is_empty() {
        parts.push(format!("ALIASES:\n{}", help.alias.join(", ")));
    }
    parts.join("\n\n")
}

pub fn eval_help(args: &[Expr]) -> Expr {
    match args {
        [] => {
            // List all primary topics (skip aliases by deduplicating via name).
            let mut names: Vec<String> = help_map()
                .values()
                .map(|h| h.name.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            names.sort();
            Expr::String(names.join(", ").into())
        }
        [first] | [first, _] => {
            let key = format_for_print(first).trim().to_lowercase();
            let section_name = args.get(1).map(|e| format_for_print(e).trim().to_lowercase());
            match help_map().get(&key) {
                Some(help) => Expr::String(
                    render_help(&key, help, section_name.as_deref()).into(),
                ),
                None => Expr::String(format!("help: no documentation for '{}'", key).into()),
            }
        }
        _ => Expr::String("help: usage: help() or help(\"name\") or help(\"name\", \"section\"".into()),
    }
}

fn format_for_print(expr: &Expr) -> String {
    match expr {
        Expr::String(s) => s.to_string(),
        Expr::Symbol(id) => resolve(*id).to_string(),
        _ => expr.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_loads_factor() {
        let map = help_map();
        assert!(map.contains_key("factor"));
        let f = &map["factor"];
        assert_eq!(f.name, "factor");
        assert!(!f.description.is_empty());
    }

    #[test]
    fn help_lists_topics() {
        let result = eval_help(&[]);
        assert!(result.to_string().contains("factor"));
    }

    #[test]
    fn help_unknown() {
        let result = eval_help(&[Expr::sym("not_a_real_function")]);
        assert!(result.to_string().contains("no documentation"));
    }

    #[test]
    fn help_loads_all_entries() {
        let map = help_map();
        assert!(map.len() >= 295, "expected at least 295 help entries, got {}", map.len());
    }

    #[test]
    fn help_tier1_entries_are_rich() {
        let map = help_map();
        let tier1 = [
            "factor", "diff", "integrate", "expand", "simplify", "solve", "limit", "sum",
            "matrix", "determinant", "eigenvalues", "plot2d", "laplace", "ode2", "gamma",
        ];
        for name in tier1 {
            let h = map.get(name).unwrap_or_else(|| panic!("missing help for {}", name));
            assert!(!h.arguments.is_empty(), "{} missing arguments", name);
            assert!(!h.details.is_empty(), "{} missing details", name);
            assert!(!h.value.is_empty(), "{} missing value", name);
        }
    }

    #[test]
    fn help_aliases_resolve() {
        let map = help_map();
        assert!(map.contains_key("create_list"));
        assert_eq!(map["create_list"].name, "makelist");
        assert!(map.contains_key("ratdenom"));
        assert_eq!(map["ratdenom"].name, "denom");
    }

    #[test]
    fn help_section_queries() {
        let r = eval_help(&[Expr::sym("factor"), Expr::sym("usage")]);
        assert!(r.to_string().contains("factor("));
        let r = eval_help(&[Expr::sym("diff"), Expr::sym("arguments")]);
        assert!(r.to_string().contains("expr"));
    }
}
