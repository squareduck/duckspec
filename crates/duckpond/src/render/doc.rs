use crate::artifact::doc::*;
use crate::render::render_body;

impl Document {
    /// Render the document to canonical markdown.
    pub fn render(&self) -> String {
        let mut out = String::new();

        // H1 + summary
        out.push_str(&format!("# {}\n\n{}", self.title, self.summary));

        // Description
        if !self.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&render_body(&self.description));
        }

        // Sections
        for section in &self.sections {
            out.push_str("\n\n");
            render_section(&mut out, section);
        }

        out.push('\n');
        out
    }
}

fn render_section(out: &mut String, section: &Section) {
    let hashes = "#".repeat(section.level as usize);
    out.push_str(&format!("{hashes} {}", section.heading));

    if !section.body.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&section.body));
    }

    for child in &section.children {
        out.push_str("\n\n");
        render_section(out, child);
    }
}
