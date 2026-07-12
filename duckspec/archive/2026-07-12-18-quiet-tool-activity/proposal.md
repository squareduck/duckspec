# Quiet tool activity

Calm the chat transcript so tool/activity groups read like Thinking: secondary chrome, not
framed cards — leaving only user input and agent answers as the prominent surfaces.

## Motivation

After recent UI work, tool cards in the chat log dominate: bordered frames, filled
headers, and paper bodies compete with answers. Thinking is already a flat muted
collapsible header; Activity is not. The transcript’s intended hierarchy (answer primary,
thinking secondary, tools tertiary) fails at a glance.

Why now: the quiet Thinking treatment is established and the card chrome is the remaining
noise.

## Intent

- Collapsed and expanded Activity headers share Thinking’s flat presentation — no bordered
  card frame or filled header/body surface for the group

- Only user messages and agent answer prose stay visually primary

- Thinking and Activity stay disambiguated by their labels (line count vs tool count and
  names), without extra icon chrome

- Expanded tool rows stay quiet detail under the header (status + summary + truncated
  output), not nested cards

- Existing collapse defaults and summary labels (line counts / tool counts and names)
  remain the calm pattern; this change is paint and hierarchy, not segment semantics

## Non-goals

- Changing how tools pair, group, or auto-collapse
- Redesigning Answer or User card treatment beyond leaving them primary
- Mid-turn choice chips, meta cards, or other non-tool chrome
- Per-tool expand/collapse or new activity interaction models
