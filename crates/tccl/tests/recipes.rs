//! Every `examples/*.scenario` (the tested recipes of the documentation) must pass.

#[test]
fn all_recipes_pass() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|x| x != "scenario") {
            continue;
        }
        let script = std::fs::read_to_string(&path).unwrap();
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"));
        let report = tccl::scenario::run(&script, &read);
        assert!(report.ok(), "{}:\n{}\n{}", path.display(), report.failed.join("\n"), report.log.join("\n"));
        assert!(report.passed > 0, "{} checks nothing", path.display());
        count += 1;
    }
    assert!(count >= 10, "expected the recipe scenarios, found {count}");
}
