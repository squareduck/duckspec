use crate::artifact::doc::*;
use crate::config::FormatConfig;
use crate::format::prose;
use crate::render::render_body;

impl Document {
    /// Render the document to canonical markdown using default formatting.
    pub fn render(&self) -> String {
        self.render_with(&FormatConfig::default())
    }

    /// Render the document to canonical markdown with the given formatting config.
    pub fn render_with(&self, config: &FormatConfig) -> String {
        let width = config.line_width;
        let mut out = String::new();

        // H1 + summary
        out.push_str(&format!(
            "# {}\n\n{}",
            self.title,
            prose::reflow(&self.summary, width)
        ));

        // Description
        if !self.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&render_body(&self.description, width));
        }

        // Sections
        for section in &self.sections {
            out.push_str("\n\n");
            render_section(&mut out, section, width);
        }

        out.push('\n');
        out
    }
}

fn render_section(out: &mut String, section: &Section, width: usize) {
    let hashes = "#".repeat(section.level as usize);
    out.push_str(&format!("{hashes} {}", section.heading));

    if !section.body.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&section.body, width));
    }

    for child in &section.children {
        out.push_str("\n\n");
        render_section(out, child, width);
    }
}
