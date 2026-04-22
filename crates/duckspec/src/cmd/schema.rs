use std::fs;

const SCHEMA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content/schemas");

pub fn run(name: String) -> anyhow::Result<()> {
    let schema_path = format!("{SCHEMA_DIR}/{name}.md");
    let content =
        fs::read_to_string(&schema_path).map_err(|_| anyhow::anyhow!("unknown schema: {name}"))?;
    print!("{content}");
    Ok(())
}
