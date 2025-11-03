#[test]
fn compile_fail_missing_scene() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fixtures/compose_missing_scene.rs");
}
