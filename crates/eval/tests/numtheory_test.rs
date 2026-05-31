use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn ifactors_360() { assert_eq!(run("ifactors(360);"), "[[2,3],[3,2],[5,1]]"); }
#[test] fn ifactors_prime() { assert_eq!(run("ifactors(17);"), "[[17,1]]"); }
#[test] fn ifactors_1() { assert_eq!(run("ifactors(1);"), "[]"); }
#[test] fn totient_12() { assert_eq!(run("totient(12);"), "4"); }
#[test] fn totient_prime() { assert_eq!(run("totient(13);"), "12"); }
#[test] fn divisors_12() { assert_eq!(run("divisors(12);"), "[1,2,3,4,6,12]"); }
#[test] fn divisors_prime() { assert_eq!(run("divisors(7);"), "[1,7]"); }
#[test] fn next_prime_100() { assert_eq!(run("next_prime(100);"), "101"); }
#[test] fn next_prime_2() { assert_eq!(run("next_prime(1);"), "2"); }
#[test] fn prev_prime_100() { assert_eq!(run("prev_prime(100);"), "97"); }
#[test] fn power_mod() { assert_eq!(run("power_mod(2, 100, 1000000007);"), "976371285"); }
#[test] fn power_mod_neg() { assert_eq!(run("power_mod(3, -1, 7);"), "5"); }
#[test] fn inv_mod() { assert_eq!(run("inv_mod(3, 7);"), "5"); }
#[test] fn jacobi_sym() { assert_eq!(run("jacobi(2, 7);"), "1"); }
#[test] fn jacobi_sym2() { assert_eq!(run("jacobi(5, 21);"), "1"); }
#[test] fn chinese() { assert_eq!(run("chinese([2,3,2],[3,5,7]);"), "23"); }
#[test] fn fibonacci_0() { assert_eq!(run("fibonacci(0);"), "0"); }
#[test] fn fibonacci_1() { assert_eq!(run("fibonacci(1);"), "1"); }
#[test] fn fibonacci_10() { assert_eq!(run("fibonacci(10);"), "55"); }
#[test] fn fibonacci_50() { assert_eq!(run("fibonacci(50);"), "12586269025"); }
// Regression: fib(100) overflows i64; must use BigInt (was silently wrong)
#[test] fn fibonacci_100() { assert_eq!(run("fibonacci(100);"), "354224848179261915075"); }
#[test] fn fibonacci_92() { assert_eq!(run("fibonacci(92);"), "7540113804746346429"); }
