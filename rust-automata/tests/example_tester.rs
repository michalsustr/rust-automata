use std::process::Command;

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}

#[test]
fn asm_generation() {
    let output = Command::new("cargo")
        .args(&["asm", "--no-color", "flip_flop::main"])
        .output()
        .expect("Failed to execute `cargo asm` -- is it installed?");

    // Check it matches the expected output
    let expected_output = std::fs::read_to_string("tests/asm/flip_flop.stdout")
        .expect("Failed to read expected test output");
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected_output);
}
