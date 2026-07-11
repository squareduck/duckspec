//! System slash-command registry, completion-catalog merge, and submit routing.
//!
//! Duckboard owns local System commands; harness discovery contributes
//! Workflow (`ds-*`) and Agent entries. See `chat/slash-commands`.

use duckchat::{SlashCommand, SlashCommandKind};

/// How a composer submit should be routed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitSlash {
    /// Bare system command (v1: `/help`) — local handler, no agent turn.
    LocalHelp,
    /// Agent turn. `display` is the user-bubble text; `prompt` is what the
    /// harness receives (may differ under `//` escape).
    Agent { display: String, prompt: String },
}

/// Duckboard-local system commands (v1: `/help` only).
pub fn system_registry() -> Vec<SlashCommand> {
    vec![SlashCommand {
        name: "help".into(),
        description: "Duckboard help (local). Agent: //help".into(),
        kind: SlashCommandKind::System,
    }]
}

/// True when `name` (no leading `/`) is in the system registry.
pub fn is_system_command_name(name: &str) -> bool {
    system_registry().iter().any(|c| c.name == name)
}

/// Agent prompt for re-dispatching a transcript user message (lost-session
/// recovery). `None` when the message was a local system command (no agent).
pub fn agent_prompt_for_recovery(user_text: &str) -> Option<String> {
    match parse_submit_slash(user_text) {
        SubmitSlash::LocalHelp => None,
        SubmitSlash::Agent { prompt, .. } => Some(prompt),
    }
}

/// Parse a composer submit into local system vs agent (with optional `//` escape).
pub fn parse_submit_slash(text: &str) -> SubmitSlash {
    let trimmed = text.trim();

    // Double-slash escape: bare `//name` → agent prompt `/name`, display kept.
    if let Some(rest) = trimmed.strip_prefix("//")
        && !rest.is_empty()
        && !rest.chars().any(char::is_whitespace)
    {
        return SubmitSlash::Agent {
            display: trimmed.to_string(),
            prompt: format!("/{rest}"),
        };
    }

    // Bare single-slash system command (v1: help only).
    if crate::chat_store::is_bare_slash_command(trimmed) {
        let name = trimmed.trim_start_matches('/');
        if is_system_command_name(name) {
            return SubmitSlash::LocalHelp;
        }
    }

    SubmitSlash::Agent {
        display: text.to_string(),
        prompt: text.to_string(),
    }
}

/// Fixed prefix + kind sections from the live catalog for local `/help`.
///
/// Empty kind sections are omitted. Entries within a section are sorted by name.
pub fn build_system_help_body(catalog: &[SlashCommand], harness_id: Option<&str>) -> String {
    let mut out = String::from(
        "Running system command `/help`.\n\
         For agent help (harness skill docs), use `//help`.\n",
    );

    append_kind_section(
        &mut out,
        "System (duckboard)",
        catalog,
        SlashCommandKind::System,
    );
    append_kind_section(
        &mut out,
        "Workflow (duckspec → agent)",
        catalog,
        SlashCommandKind::Workflow,
    );
    let agent_title = match harness_id {
        Some(h) if !h.is_empty() => format!("Agent skills (→ {h})"),
        _ => "Agent skills".to_string(),
    };
    append_kind_section(&mut out, &agent_title, catalog, SlashCommandKind::Agent);

    out.push_str("\n## Escape\n");
    out.push_str("`//help` — send `/help` to the agent (harness skill docs).\n");
    out
}

fn append_kind_section(
    out: &mut String,
    title: &str,
    catalog: &[SlashCommand],
    kind: SlashCommandKind,
) {
    let mut entries: Vec<&SlashCommand> = catalog.iter().filter(|c| c.kind == kind).collect();
    if entries.is_empty() {
        return;
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    out.push('\n');
    out.push_str("## ");
    out.push_str(title);
    out.push('\n');
    for e in entries {
        out.push_str("- /");
        out.push_str(&e.name);
        if !e.description.is_empty() {
            out.push_str(" — ");
            out.push_str(&e.description);
        }
        out.push('\n');
    }
}

/// Sort rank for equal fuzzy scores: System before Workflow before Agent.
pub fn slash_kind_rank(kind: SlashCommandKind) -> u8 {
    match kind {
        SlashCommandKind::System => 0,
        SlashCommandKind::Workflow => 1,
        SlashCommandKind::Agent => 2,
    }
}

/// Optional short tag for completion rows (`sys` for System only).
pub fn slash_kind_row_tag(kind: SlashCommandKind) -> Option<&'static str> {
    match kind {
        SlashCommandKind::System => Some("sys"),
        SlashCommandKind::Workflow | SlashCommandKind::Agent => None,
    }
}

/// Merge system registry with harness discovery into the completion catalog.
///
/// - System entries first (their names win on collision).
/// - Discovered names starting with `ds-` → Workflow; others → Agent.
/// - Discovered names already present as System are dropped.
pub fn build_completion_catalog(
    system: Vec<SlashCommand>,
    discovered: Vec<SlashCommand>,
) -> Vec<SlashCommand> {
    let mut catalog = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cmd in system {
        seen.insert(cmd.name.clone());
        catalog.push(SlashCommand {
            name: cmd.name,
            description: cmd.description,
            kind: SlashCommandKind::System,
        });
    }

    for cmd in discovered {
        if seen.contains(&cmd.name) {
            continue;
        }
        seen.insert(cmd.name.clone());
        let kind = if cmd.name.starts_with("ds-") {
            SlashCommandKind::Workflow
        } else {
            SlashCommandKind::Agent
        };
        catalog.push(SlashCommand {
            name: cmd.name,
            description: cmd.description,
            kind,
        });
    }

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sys(name: &str) -> SlashCommand {
        SlashCommand {
            name: name.into(),
            description: format!("{name} system"),
            kind: SlashCommandKind::System,
        }
    }

    fn discovered(name: &str) -> SlashCommand {
        // Discovery leaves kind as Agent; merge re-tags.
        SlashCommand {
            name: name.into(),
            description: format!("{name} disc"),
            kind: SlashCommandKind::Agent,
        }
    }

    /// @spec chat/slash-commands Kinded completion catalog: System registry entries are System
    #[test]
    fn system_registry_entries_are_system() {
        // GIVEN a system-registry command named `help`
        let system = vec![sys("help")];
        // WHEN the completion catalog is built
        let catalog = build_completion_catalog(system, vec![]);
        // THEN the catalog includes an entry named `help` with kind System
        let help = catalog.iter().find(|c| c.name == "help").expect("help");
        assert_eq!(help.kind, SlashCommandKind::System);
    }

    /// @spec chat/slash-commands Kinded completion catalog: Discovered ds-* names are Workflow
    #[test]
    fn discovered_ds_names_are_workflow() {
        // GIVEN harness discovery returns a command named `ds-spec`
        let discovered = vec![discovered("ds-spec")];
        // WHEN the completion catalog is built
        let catalog = build_completion_catalog(vec![], discovered);
        // THEN kind is Workflow
        let entry = catalog.iter().find(|c| c.name == "ds-spec").expect("ds-spec");
        assert_eq!(entry.kind, SlashCommandKind::Workflow);
    }

    /// @spec chat/slash-commands Kinded completion catalog: Other discovered names are Agent
    #[test]
    fn other_discovered_names_are_agent() {
        // GIVEN harness discovery returns a command named `review` not in system registry
        let discovered = vec![discovered("review")];
        // WHEN the completion catalog is built
        let catalog = build_completion_catalog(vec![], discovered);
        // THEN kind is Agent
        let entry = catalog.iter().find(|c| c.name == "review").expect("review");
        assert_eq!(entry.kind, SlashCommandKind::Agent);
    }

    /// @spec chat/slash-commands Kinded completion catalog: System name wins on collision with discovery
    #[test]
    fn system_name_wins_on_collision_with_discovery() {
        // GIVEN system `help` and discovery also returns `help`
        let system = vec![sys("help")];
        let disc = vec![discovered("help")];
        // WHEN the completion catalog is built
        let catalog = build_completion_catalog(system, disc);
        // THEN exactly one `help`, kind System
        let helps: Vec<_> = catalog.iter().filter(|c| c.name == "help").collect();
        assert_eq!(helps.len(), 1);
        assert_eq!(helps[0].kind, SlashCommandKind::System);
        assert_eq!(helps[0].description, "help system");
    }

    /// @spec chat/slash-commands Kinded completion catalog: Claude interactive builtins are not Agent catalog entries
    #[test]
    fn claude_interactive_builtins_are_not_agent_catalog_entries() {
        // GIVEN discovery with no Claude interactive builtins (shared scanner cleaned)
        // and no system overrides for clear/compact/cost/model
        let system = system_registry(); // only `help` as System
        let discovered = vec![discovered("review")];
        // WHEN the completion catalog is built
        let catalog = build_completion_catalog(system, discovered);
        // THEN no Agent entry for clear/compact/cost/help/model as fake builtins
        for name in ["clear", "compact", "cost", "model"] {
            assert!(
                catalog.iter().all(|c| c.name != name),
                "must not list builtin `{name}`"
            );
        }
        let help = catalog.iter().find(|c| c.name == "help").expect("help");
        assert_eq!(
            help.kind,
            SlashCommandKind::System,
            "help must be System from registry, not Agent builtin"
        );
        assert!(
            catalog
                .iter()
                .any(|c| c.name == "review" && c.kind == SlashCommandKind::Agent)
        );
    }

    /// @spec chat/slash-commands Local system submit: Bare /help does not start an agent turn
    #[test]
    fn bare_help_routes_local_not_agent() {
        // GIVEN a chat session ready to send
        // WHEN the user submits bare `/help`
        let route = parse_submit_slash("/help");
        // THEN no agent turn is started for that submit (local route)
        assert_eq!(route, SubmitSlash::LocalHelp);
        assert_eq!(parse_submit_slash("  /help  "), SubmitSlash::LocalHelp);
    }

    /// @spec chat/slash-commands Double-slash agent escape: Bare //help is an agent turn with prompt /help
    #[test]
    fn bare_double_slash_help_is_agent_with_prompt_help() {
        // GIVEN a chat session ready to send
        // WHEN the user submits bare `//help`
        let route = parse_submit_slash("//help");
        // THEN an agent turn is started AND the turn prompt is `/help`
        match route {
            SubmitSlash::Agent { prompt, .. } => assert_eq!(prompt, "/help"),
            other => panic!("expected agent route, got {other:?}"),
        }
    }

    #[test]
    fn recovery_prompt_for_double_slash_help_is_single_slash() {
        // Lost-session recovery must re-parse display `//help` → agent `/help`.
        assert_eq!(
            agent_prompt_for_recovery("//help").as_deref(),
            Some("/help")
        );
        assert_eq!(agent_prompt_for_recovery("/help"), None);
        assert_eq!(
            agent_prompt_for_recovery("hello").as_deref(),
            Some("hello")
        );
    }

    /// @spec chat/slash-commands Double-slash agent escape: Escape keeps typed //help as the user message text
    #[test]
    fn escape_keeps_typed_double_slash_as_display() {
        // GIVEN a chat session ready to send
        // WHEN the user submits bare `//help`
        let route = parse_submit_slash("//help");
        // THEN the user message text recorded for that submit is `//help`
        match route {
            SubmitSlash::Agent { display, .. } => assert_eq!(display, "//help"),
            other => panic!("expected agent route, got {other:?}"),
        }
    }

    /// @spec chat/slash-commands Local system submit: System reply prefix names the command and teaches //help
    #[test]
    fn system_help_prefix_names_command_and_teaches_escape() {
        // GIVEN a chat session ready to send
        // WHEN help body is built (as for bare `/help`)
        let body = build_system_help_body(&[], None);
        // THEN prefix states system command `/help` is running and teaches `//help`
        assert!(
            body.contains("Running system command `/help`."),
            "missing running notice: {body}"
        );
        assert!(
            body.contains("`//help`") || body.contains("use `//help`"),
            "missing escape guidance: {body}"
        );
    }

    /// @spec chat/slash-commands Local system submit: Help body lists non-empty kind sections from the live catalog
    #[test]
    fn help_body_lists_non_empty_kind_sections_only() {
        // GIVEN catalog with System + Workflow, no Agent
        let catalog = vec![
            sys("help"),
            SlashCommand {
                name: "ds-spec".into(),
                description: "Spec stage".into(),
                kind: SlashCommandKind::Workflow,
            },
        ];
        // WHEN help body is built
        let body = build_system_help_body(&catalog, Some("grok"));
        // THEN System and Workflow sections present; Agent section omitted
        assert!(body.contains("## System (duckboard)"), "{body}");
        assert!(body.contains("/help"), "{body}");
        assert!(body.contains("## Workflow (duckspec → agent)"), "{body}");
        assert!(body.contains("/ds-spec"), "{body}");
        assert!(
            !body.contains("## Agent skills"),
            "empty agent section must be omitted: {body}"
        );
    }

    #[test]
    fn slash_kind_rank_orders_system_workflow_agent() {
        assert!(slash_kind_rank(SlashCommandKind::System) < slash_kind_rank(SlashCommandKind::Workflow));
        assert!(
            slash_kind_rank(SlashCommandKind::Workflow) < slash_kind_rank(SlashCommandKind::Agent)
        );
    }
}
