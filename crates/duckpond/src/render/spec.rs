use crate::artifact::spec::*;
use crate::config::FormatConfig;
use crate::format::prose;
use crate::render::render_body;

impl Spec {
    /// Render the spec to canonical markdown using default formatting.
    pub fn render(&self) -> String {
        self.render_with(&FormatConfig::default())
    }

    /// Render the spec to canonical markdown with the given formatting config.
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

        // Requirements
        for req in &self.requirements {
            out.push_str("\n\n");
            render_requirement(&mut out, req, width);
        }

        out.push('\n');
        out
    }
}

fn render_requirement(out: &mut String, req: &Requirement, width: usize) {
    out.push_str(&format!("## Requirement: {}", req.name));

    // Prose
    if !req.prose.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&req.prose, width));
    }

    // Requirement-level test marker
    if let Some(marker) = &req.test_marker {
        out.push_str("\n\n");
        render_test_marker(out, marker);
    }

    // Scenarios
    for scenario in &req.scenarios {
        out.push_str("\n\n");
        render_scenario(out, scenario, width);
    }
}

fn render_scenario(out: &mut String, scenario: &Scenario, width: usize) {
    out.push_str(&format!("### Scenario: {}", scenario.name));

    // Collect every clause (kw, text) in source order.
    let mut clauses: Vec<(&str, &str)> = Vec::new();
    for (i, c) in scenario.givens.iter().enumerate() {
        clauses.push((if i == 0 { "GIVEN" } else { "AND" }, c.text.as_str()));
    }
    for (i, c) in scenario.whens.iter().enumerate() {
        clauses.push((if i == 0 { "WHEN" } else { "AND" }, c.text.as_str()));
    }
    for (i, c) in scenario.thens.iter().enumerate() {
        clauses.push((if i == 0 { "THEN" } else { "AND" }, c.text.as_str()));
    }

    if !clauses.is_empty() {
        // Pre-render each bullet body so we can detect multi-line items and
        // apply the loose-list rule (blank line between every clause when
        // any clause spans multiple lines).
        let bodies: Vec<String> = clauses
            .iter()
            .map(|(kw, text)| render_gwt_clause_body(kw, text, width))
            .collect();
        let loose = bodies.iter().any(|s| s.contains('\n'));
        let sep = if loose { "\n\n" } else { "\n" };

        // Blank line below the H3 before the first bullet.
        out.push('\n');
        for (i, body) in bodies.iter().enumerate() {
            out.push_str(if i == 0 { "\n" } else { sep });
            out.push_str(body);
        }
    }

    // Scenario-level test marker
    if let Some(marker) = &scenario.test_marker {
        out.push_str("\n\n");
        render_test_marker(out, marker);
    }
}

/// Render a single `- **KW** text` GWT bullet (without any leading newline),
/// reflowing the text and re-indenting continuations to align under the
/// bullet content (2 spaces).
fn render_gwt_clause_body(kw: &str, text: &str, width: usize) -> String {
    let prefix = format!("- **{kw}** ");
    let prefix_len = prefix.chars().count();
    let avail = width.saturating_sub(prefix_len).max(1);
    let reflowed = prose::reflow(text, avail);
    let mut out = prefix;
    for (i, line) in reflowed.split('\n').enumerate() {
        if i > 0 {
            out.push_str("\n  ");
        }
        out.push_str(line);
    }
    out
}

fn render_test_marker(out: &mut String, marker: &TestMarker) {
    match &marker.kind {
        TestMarkerKind::Code { backlinks } => {
            out.push_str("> test: code");
            for link in backlinks {
                out.push_str(&format!("\n> - {}", link.path));
            }
        }
        TestMarkerKind::Manual { reason } => {
            out.push_str(&format!("> manual: {reason}"));
        }
        TestMarkerKind::Skip { reason } => {
            out.push_str(&format!("> skip: {reason}"));
        }
    }
}
