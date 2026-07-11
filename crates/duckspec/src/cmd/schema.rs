use crate::content;

pub fn run(name: String) -> anyhow::Result<()> {
    let body =
        content::schema(&name).ok_or_else(|| anyhow::anyhow!("unknown schema: {name}"))?;
    print!("{body}");
    Ok(())
}
