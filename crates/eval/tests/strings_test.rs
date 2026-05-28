use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn slength() { assert_eq!(run(r#"slength("hello");"#), "5"); }
#[test] fn slength_empty() { assert_eq!(run(r#"slength("");"#), "0"); }
#[test] fn charat() { assert_eq!(run(r#"charat("hello", 2);"#), "\"e\""); }
#[test] fn charat_first() { assert_eq!(run(r#"charat("abc", 1);"#), "\"a\""); }
#[test] fn substring_range() { assert_eq!(run(r#"substring("hello", 2, 4);"#), "\"ell\""); }
#[test] fn substring_to_end() { assert_eq!(run(r#"substring("hello", 3);"#), "\"llo\""); }
#[test] fn ssearch_found() { assert_eq!(run(r#"ssearch("ll", "hello");"#), "3"); }
#[test] fn ssearch_not_found() { assert_eq!(run(r#"ssearch("xyz", "hello");"#), "false"); }
#[test] fn ssubst() { assert_eq!(run(r#"ssubst("X", "l", "hello");"#), "\"heXXo\""); }
#[test] fn split_comma() { assert_eq!(run(r#"split("a,b,c", ",");"#), "[\"a\",\"b\",\"c\"]"); }
#[test] fn split_space() { assert_eq!(run(r#"split("one two three");"#), "[\"one\",\"two\",\"three\"]"); }
#[test] fn supcase() { assert_eq!(run(r#"supcase("hello");"#), "\"HELLO\""); }
#[test] fn sdowncase() { assert_eq!(run(r#"sdowncase("HELLO");"#), "\"hello\""); }
#[test] fn strim() { assert_eq!(run(r#"strim("  hi  ");"#), "\"hi\""); }
#[test] fn sequal_true() { assert_eq!(run(r#"sequal("abc", "abc");"#), "true"); }
#[test] fn sequal_false() { assert_eq!(run(r#"sequal("abc", "def");"#), "false"); }
#[test] fn parse_string() {
    let r = run(r#"parse_string("x^2+1");"#);
    assert!(r.contains("x") && r.contains("1"), "got: {}", r);
}
