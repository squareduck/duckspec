# @ Chat transcript

## ~ Activity groups

Consecutive tools form one Activity segment. Inside the group, uses and results pair by
call id, not by adjacency alone — so interleaved completions still merge into one row per
call. A completed row carries a short tool summary and the result body. A result with no
matching use still becomes a done row labeled from the tool name; it is never shown only
as a bare "done" placeholder.

Structured host-choice tools (AskUserQuestion and equivalents) are omitted from Activity
entirely — mid-turn questions use fast-response chips, not Activity rows. Ordinary tools
in the same stream still group as usual.

Activity uses the same flat secondary chrome as Thinking: a collapsible muted header with
no bordered card frame and no filled header or body surface. When expanded, each tool is
one quiet row (status + summary) with truncated output under the row when present.
Collapse is group-level only: there is no nested per-tool expand state.

Collapsed, the group summarizes as a count plus sample tool names (for example
`4 tools · Read, grep, shell`).

## ~ Live vs settled

```
LIVE                                      SETTLED
────                                      ──────
Thinking open (streaming body, faded)     Thinking collapsed (line count)
Activity expanded (quiet rows, faded)     Activity collapsed (count · names)
Answer streaming as plain prose           Answer as plain prose
```

Thinking and expanded Activity body ink is slightly more faded than Answer prose
(secondary text color) so supporting work stays quiet. Headers are more muted still.
Harnesses that never emit reasoning simply never open Thinking segments; Activity and
Answer behavior still apply. Presentation is driven only from neutral session content and
stream buffers — not from harness-specific UI branches.

## + Secondary chrome

User messages keep a paper card and Answer prose stays primary on the chat background.
Thinking and Activity share one secondary presentation: flat collapsible header (chevron
and muted label) and no group-level card chrome. Labels alone disambiguate the two kinds
(`Thinking · N lines` vs `N tools · …`).

```
| Segment  | Chrome                         | Role            |
| -------- | ------------------------------ | --------------- |
| User     | paper card                     | primary object  |
| Answer   | plain / last-answer band       | primary reply   |
| Thinking | flat header (muted label)      | secondary (why) |
| Activity | flat header + quiet tool rows  | tertiary (what) |
```
