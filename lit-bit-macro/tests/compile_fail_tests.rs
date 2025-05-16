#[test]
fn compile_tests() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/compile-fail/unknown_target_state.rs");
    // Add other compile-fail test cases here if needed in the future
    // For example, if we add more specific compile-time errors:
    // test_cases.compile_fail("tests/compile-fail/another_error_case.rs");
}
