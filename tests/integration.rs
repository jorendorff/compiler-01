use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Compile a Toy program and run it, returning its stdout.
fn run_toy(source: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("toy_test_{}", id));
    fs::create_dir_all(&tmp_dir).unwrap();

    let src_path = tmp_dir.join("test.toy");
    let exe_path = tmp_dir.join("test_exe");

    fs::write(&src_path, source).unwrap();

    // Build the compiler first (cargo build should be a no-op if already built)
    let compiler_path = PathBuf::from(env!("CARGO_BIN_EXE_toy-compiler"));

    // Run the compiler
    let compile_output = Command::new(&compiler_path)
        .args([
            src_path.to_str().unwrap(),
            "-o",
            exe_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run toy-compiler");

    assert!(
        compile_output.status.success(),
        "Compilation failed for program:\n{}\nstderr: {}",
        source,
        String::from_utf8_lossy(&compile_output.stderr)
    );

    // Run the compiled executable
    let run_output = Command::new(&exe_path)
        .output()
        .expect("failed to run compiled program");

    assert!(
        run_output.status.success(),
        "Execution failed for program:\n{}\nstderr: {}",
        source,
        String::from_utf8_lossy(&run_output.stderr)
    );

    // Clean up
    let _ = fs::remove_dir_all(&tmp_dir);

    String::from_utf8(run_output.stdout).unwrap()
}

/// Compile a Toy program and expect compilation to fail.
fn expect_compile_error(source: &str) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("toy_test_{}", id));
    fs::create_dir_all(&tmp_dir).unwrap();

    let src_path = tmp_dir.join("test.toy");
    let exe_path = tmp_dir.join("test_exe");

    fs::write(&src_path, source).unwrap();

    let compiler_path = PathBuf::from(env!("CARGO_BIN_EXE_toy-compiler"));

    let compile_output = Command::new(&compiler_path)
        .args([
            src_path.to_str().unwrap(),
            "-o",
            exe_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run toy-compiler");

    let _ = fs::remove_dir_all(&tmp_dir);

    assert!(
        !compile_output.status.success(),
        "Expected compilation to fail for program:\n{}",
        source,
    );
}

// ==================== Arithmetic tests ====================

#[test]
fn simple_integer() {
    assert_eq!(run_toy("print 42;"), "42\n");
}

#[test]
fn zero() {
    assert_eq!(run_toy("print 0;"), "0\n");
}

#[test]
fn addition() {
    assert_eq!(run_toy("print 2 + 3;"), "5\n");
}

#[test]
fn subtraction() {
    assert_eq!(run_toy("print 10 - 3;"), "7\n");
}

#[test]
fn multiplication() {
    assert_eq!(run_toy("print 6 * 7;"), "42\n");
}

#[test]
fn division() {
    assert_eq!(run_toy("print 7 / 2;"), "3\n");
}

#[test]
fn modulo() {
    assert_eq!(run_toy("print 10 % 3;"), "1\n");
}

#[test]
fn precedence_mul_over_add() {
    assert_eq!(run_toy("print 2 + 3 * 4;"), "14\n");
}

#[test]
fn precedence_mul_over_sub() {
    assert_eq!(run_toy("print 10 - 2 * 3;"), "4\n");
}

#[test]
fn parentheses_override_precedence() {
    assert_eq!(run_toy("print (2 + 3) * 4;"), "20\n");
}

#[test]
fn left_associative_subtraction() {
    assert_eq!(run_toy("print 10 - 3 - 2;"), "5\n");
}

#[test]
fn left_associative_division() {
    assert_eq!(run_toy("print 24 / 4 / 2;"), "3\n");
}

#[test]
fn nested_parentheses() {
    assert_eq!(run_toy("print ((2 + 3) * (4 + 1));"), "25\n");
}

#[test]
fn complex_expression() {
    // 1 + (2*3) - (4/2) + 5 = 1 + 6 - 2 + 5 = 10
    assert_eq!(run_toy("print 1 + 2 * 3 - 4 / 2 + 5;"), "10\n");
}

// ==================== Unary minus tests ====================

#[test]
fn unary_minus() {
    assert_eq!(run_toy("print -5;"), "-5\n");
}

#[test]
fn unary_minus_in_expression() {
    assert_eq!(run_toy("print -5 + 3;"), "-2\n");
}

#[test]
fn double_negation() {
    assert_eq!(run_toy("print --5;"), "5\n");
}

#[test]
fn unary_minus_with_parens() {
    assert_eq!(run_toy("print -(3 + 4);"), "-7\n");
}

#[test]
fn unary_minus_precedence() {
    // Unary minus binds tighter than *, so this is (-2) * 3 = -6
    assert_eq!(run_toy("print -2 * 3;"), "-6\n");
}

// ==================== Variable tests ====================

#[test]
fn simple_variable() {
    assert_eq!(run_toy("let x = 10;\nprint x;"), "10\n");
}

#[test]
fn two_variables() {
    assert_eq!(
        run_toy("let x = 10;\nlet y = 3;\nprint x - y;"),
        "7\n"
    );
}

#[test]
fn variable_in_expression() {
    assert_eq!(
        run_toy("let x = 5;\nlet y = 3;\nprint x * y + 1;"),
        "16\n"
    );
}

#[test]
fn reassignment() {
    assert_eq!(run_toy("let x = 1;\nx = 2;\nprint x;"), "2\n");
}

#[test]
fn reassignment_with_self() {
    assert_eq!(run_toy("let x = 5;\nx = x + 1;\nprint x;"), "6\n");
}

#[test]
fn shadowing() {
    assert_eq!(run_toy("let x = 1;\nlet x = 2;\nprint x;"), "2\n");
}

#[test]
fn shadowing_with_self_reference() {
    assert_eq!(
        run_toy("let x = 5;\nlet x = x + 1;\nprint x;"),
        "6\n"
    );
}

#[test]
fn multiple_variables() {
    let src = "\
let a = 1;
let b = 2;
let c = 3;
let d = a + b + c;
print d;
";
    assert_eq!(run_toy(src), "6\n");
}

// ==================== Print tests ====================

#[test]
fn multiple_prints() {
    assert_eq!(
        run_toy("print 1;\nprint 2;\nprint 3;"),
        "1\n2\n3\n"
    );
}

#[test]
fn print_variable_and_expression() {
    let src = "\
let x = 10;
print x;
print x + 5;
";
    assert_eq!(run_toy(src), "10\n15\n");
}

// ==================== Overflow / wrapping tests ====================

#[test]
fn overflow_wraps_to_negative() {
    assert_eq!(
        run_toy("print 9223372036854775807 + 1;"),
        "-9223372036854775808\n"
    );
}

#[test]
fn underflow_wraps_to_positive() {
    // -9223372036854775808 - 1 should wrap to 9223372036854775807
    // We express -9223372036854775808 as 0 - 9223372036854775807 - 1
    let src = "\
let x = 0 - 9223372036854775807 - 1;
print x - 1;
";
    assert_eq!(run_toy(src), "9223372036854775807\n");
}

// ==================== Division semantics ====================

#[test]
fn division_truncates_toward_zero_positive() {
    assert_eq!(run_toy("print 7 / 2;"), "3\n");
}

#[test]
fn division_truncates_toward_zero_negative() {
    assert_eq!(run_toy("print -7 / 2;"), "-3\n");
}

#[test]
fn modulo_sign_matches_dividend() {
    assert_eq!(run_toy("print -7 % 3;"), "-1\n");
}

#[test]
fn modulo_positive() {
    assert_eq!(run_toy("print 7 % -3;"), "1\n");
}

// ==================== Comment tests ====================

#[test]
fn line_comment() {
    let src = "\
// This is a comment
print 42; // inline comment
";
    assert_eq!(run_toy(src), "42\n");
}

#[test]
fn comment_only_program() {
    // A program with only comments and no statements should produce no output
    assert_eq!(run_toy("// nothing here\n"), "");
}

// ==================== Larger programs ====================

#[test]
fn fibonacci_sequence() {
    let src = "\
let a = 0;
let b = 1;
print a;
print b;
let c = a + b;
print c;
let a = b;
let b = c;
let c = a + b;
print c;
let a = b;
let b = c;
let c = a + b;
print c;
";
    assert_eq!(run_toy(src), "0\n1\n1\n2\n3\n");
}

// ==================== Large integer literal ====================

#[test]
fn large_positive_literal() {
    assert_eq!(run_toy("print 9223372036854775807;"), "9223372036854775807\n");
}

#[test]
fn large_negative_via_unary() {
    assert_eq!(run_toy("print -9223372036854775807;"), "-9223372036854775807\n");
}

// ==================== Empty program ====================

#[test]
fn empty_program() {
    assert_eq!(run_toy(""), "");
}

// ==================== Tokenization / whitespace tests ====================

#[test]
fn no_spaces_around_operators() {
    assert_eq!(run_toy("print 3+4;"), "7\n");
}

#[test]
fn no_spaces_around_multiple_operators() {
    assert_eq!(run_toy("print 2+3*4;"), "14\n");
}

#[test]
fn no_spaces_in_variable_assignment() {
    assert_eq!(run_toy("let x=10;\nprint x;"), "10\n");
}

#[test]
fn no_space_between_print_and_paren() {
    // print(3) — the ( terminates the keyword, so this is valid
    assert_eq!(run_toy("print(3);"), "3\n");
}

#[test]
fn no_space_in_parenthesized_expression() {
    assert_eq!(run_toy("print(2+3)*4;"), "20\n");
}

#[test]
fn print_keyword_glued_to_digit_is_error() {
    // print3 is lexed as identifier "print3", not keyword "print" + literal 3.
    // As a statement, an identifier must be followed by =, so this is a syntax error.
    expect_compile_error("print3;");
}

#[test]
fn let_keyword_glued_to_name_is_assignment() {
    // letx=1; is lexed as identifier "letx", =, 1, ;
    // This is an assignment to "letx", which hasn't been declared.
    expect_compile_error("letx=1;");
}

#[test]
fn let_keyword_glued_to_name_with_prior_decl() {
    // If "letx" was previously declared, "letx=2;" is a valid assignment.
    assert_eq!(run_toy("let letx = 1;\nletx=2;\nprint letx;"), "2\n");
}

#[test]
fn keywords_as_prefix_of_identifier() {
    // "printing" and "letter" are valid identifiers, not keywords
    assert_eq!(
        run_toy("let printing = 5;\nlet letter = 10;\nprint printing + letter;"),
        "15\n"
    );
}

#[test]
fn all_whitespace_between_tokens() {
    // Tabs and multiple spaces work
    assert_eq!(run_toy("print\t\t3\t+\t4\t;"), "7\n");
}

#[test]
fn no_whitespace_program() {
    // Minimal whitespace: only required between "let" and identifier
    assert_eq!(run_toy("let x=1;print(x+2);"), "3\n");
}

#[test]
fn digit_followed_by_identifier() {
    // "3x" lexes as integer 3, identifier x — this is a syntax error
    // because after "print 3", the parser expects an operator or semicolon
    expect_compile_error("print 3x;");
}

// ==================== Error cases ====================

#[test]
fn error_undefined_variable() {
    expect_compile_error("print x;");
}

#[test]
fn error_assign_undeclared() {
    expect_compile_error("x = 5;");
}

#[test]
fn error_missing_semicolon() {
    expect_compile_error("print 42");
}

#[test]
fn error_unexpected_token() {
    expect_compile_error("42;");
}
