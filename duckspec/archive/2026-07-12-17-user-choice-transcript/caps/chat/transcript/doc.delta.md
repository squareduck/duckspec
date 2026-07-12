# @ Chat transcript

## @ Activity groups

Consecutive tools form one Activity segment. Inside the group, uses and results pair by
call id, not by adjacency alone — so interleaved completions still merge into one row per
call. A completed row carries a short tool summary and the result body. A result with no
matching use still becomes a done row labeled from the tool name; it is never shown only
as a bare "done" placeholder.

Structured host-choice tools (AskUserQuestion and equivalents) are omitted from Activity
entirely — mid-turn questions use fast-response chips, not tool cards. Ordinary tools in
the same stream still group as usual.

When the group is expanded, each tool is one quiet row (status + summary) with truncated
output under the row when present. Collapse is group-level only: there is no nested
per-tool expand state.

Collapsed, the group summarizes as a count plus sample tool names (for example
`4 tools · Read, grep, shell`).
