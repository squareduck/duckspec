# Completion kind cues

Show kind in the slash completion popup: distinct name colors, a `sys` tag on System rows,
and System → Workflow → Agent order when fuzzy scores tie.

## Prerequisites

- [x] @step kinded-catalog-and-discovery-cleanup

## Tasks

- [x] 1. Add pure name-color (and optional tag) helpers for `SlashCommandKind` in theme or
         agent_chat; three pairwise-distinct colors

- [x] 2. Update `view_completion_col` to color `/name` by kind and show `sys` on System
         rows

- [x] 3. Update `filter_commands` so equal scores order System before Workflow before
         Agent (score still primary)

- [x] 4. @spec chat/slash-commands Kind cues in completion: Name token color maps by kind

- [x] 5. @spec chat/slash-commands Kind cues in completion: System rows include a sys tag

- [x] 6. @spec chat/slash-commands Kind cues in completion: Equal fuzzy scores order System, Workflow, Agent
