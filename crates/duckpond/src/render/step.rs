use crate::artifact::step::*;
use crate::config::FormatConfig;
use crate::format::prose;
use crate::render::render_body;

impl Step {
    /// Render the step to canonical markdown using default formatting.
    pub fn render(&self) -> String {
        self.render_with(&FormatConfig::default())
    }

    /// Render the step to canonical markdown with the given formatting config.
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

        // Prerequisites
        if let Some(prereqs) = &self.prerequisites {
            out.push_str("\n\n## Prerequisites\n");
            for prereq in prereqs {
                let check = if prereq.checked { "x" } else { " " };
                let text = match &prereq.kind {
                    PrerequisiteKind::StepRef { slug } => format!("@step {slug}"),
                    PrerequisiteKind::Freeform { text } => text.clone(),
                };
                out.push_str(&format!("\n- [{check}] {text}"));
            }
        }

        // Context
        if let Some(ctx) = &self.context {
            out.push_str("\n\n## Context\n\n");
            out.push_str(&render_body(ctx, width));
        }

        // Tasks
        out.push_str("\n\n## Tasks\n");
        for (i, task) in self.tasks.iter().enumerate() {
            let num = i + 1;
            // Blank line between top-level tasks (not before the first).
            if i > 0 {
                out.push('\n');
            }
            render_task(&mut out, task, num, width);
            for (j, sub) in task.subtasks.iter().enumerate() {
                let sub_num = format!("{}.{}", num, j + 1);
                render_subtask(&mut out, sub, &sub_num, width);
            }
        }

        // Outcomes
        if let Some(outcomes) = &self.outcomes {
            out.push_str("\n\n## Outcomes\n\n");
            out.push_str(&render_body(outcomes, width));
        }

        out.push('\n');
        out
    }
}

/// Render a top-level task `- [ ] N. text`, hang-indenting continuations to
/// align with the column after `- [ ] N. `. `@spec` references stay on a
/// single line regardless of width.
fn render_task(out: &mut String, task: &Task, num: usize, width: usize) {
    let check = if task.checked { "x" } else { " " };
    let prefix = format!("- [{check}] {num}. ");
    render_task_line(out, &prefix, &task.content, width);
}

/// Render a subtask `  - [ ] N.M text`, hang-indenting under the same column
/// rule.
fn render_subtask(out: &mut String, sub: &Subtask, num: &str, width: usize) {
    let check = if sub.checked { "x" } else { " " };
    let prefix = format!("  - [{check}] {num} ");
    render_task_line(out, &prefix, &sub.content, width);
}

fn render_task_line(out: &mut String, prefix: &str, content: &TaskContent, width: usize) {
    out.push('\n');
    out.push_str(prefix);
    let text = render_task_content(content);
    match content {
        TaskContent::SpecRef { .. } => {
            // @spec references must stay on a single line.
            out.push_str(&text);
        }
        TaskContent::Freeform { .. } => {
            let prefix_len = prefix.chars().count();
            let avail = width.saturating_sub(prefix_len).max(1);
            let reflowed = prose::reflow(&text, avail);
            let cont_pad = " ".repeat(prefix_len);
            for (i, line) in reflowed.split('\n').enumerate() {
                if i == 0 {
                    out.push_str(line);
                } else {
                    out.push('\n');
                    out.push_str(&cont_pad);
                    out.push_str(line);
                }
            }
        }
    }
}

fn render_task_content(content: &TaskContent) -> String {
    match content {
        TaskContent::Freeform { text } => text.clone(),
        TaskContent::SpecRef {
            capability,
            requirement,
            scenario,
        } => format!("@spec {capability} {requirement}: {scenario}"),
    }
}
