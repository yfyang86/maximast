use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// Basic %i powers
#[test] fn i_squared() { assert_eq!(run("%i^2;"), "-1"); }
#[test] fn i_cubed() { assert_eq!(run("%i^3;"), "-%i"); }
#[test] fn i_fourth() { assert_eq!(run("%i^4;"), "1"); }

// realpart / imagpart on atoms
#[test] fn realpart_atom() { assert_eq!(run("realpart(3+4*%i);"), "3"); }
#[test] fn imagpart_atom() { assert_eq!(run("imagpart(3+4*%i);"), "4"); }
#[test] fn conjugate_atom() { assert_eq!(run("conjugate(3+4*%i);"), "3-4*%i"); }
#[test] fn cabs_atom() { assert_eq!(run("cabs(3+4*%i);"), "5"); }

// Regression: realpart/imagpart/cabs/conjugate must expand powers first.
// (1+%i)^2 = 2*%i, so realpart=0, imagpart=2, cabs=2, conjugate=-2*%i.
#[test] fn realpart_power() { assert_eq!(run("realpart((1+%i)^2);"), "0"); }
#[test] fn imagpart_power() { assert_eq!(run("imagpart((1+%i)^2);"), "2"); }
#[test] fn cabs_power() { assert_eq!(run("cabs((1+%i)^2);"), "2"); }
#[test] fn conjugate_power() { assert_eq!(run("conjugate((1+%i)^2);"), "-2*%i"); }

// rectform
#[test] fn rectform_power() { assert_eq!(run("rectform((1+%i)^2);"), "2*%i"); }
