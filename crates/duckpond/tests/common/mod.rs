use std::path::PathBuf;

pub fn fixture_path(category: &str, name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(category);
    path.push(name);
    path
}

pub fn load_fixture(category: &str, name: &str) -> String {
    let path = fixture_path(category, name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}
