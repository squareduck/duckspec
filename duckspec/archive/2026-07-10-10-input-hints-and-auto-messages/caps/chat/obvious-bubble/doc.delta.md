# @ Chat obvious bubble

**Auto messages:** ranked lifecycle `/ds-*` action chips plus optional affirm and decline,
with key-first labels and dual-purpose ⌘↩ — independent of under-input input hints, and
shown only when the global auto messages setting is enabled (default on).

## ~ Surfaces

Two empty-composer affordances stay separate:

```
disk phase + session empty? + VCS dirty
        │
        ├── auto messages ON
        │     obvious chrome chips + keys / click  ──▶ send action text
        │
        └── first lifecycle (empty session)
              or agent oneshot (non-empty, agent hints ON)
                    ──▶ under-input input hints (empty Enter / Tab)
```

Chrome never shows oneshot `REPLY:` lines. Under-input input hints on an empty session may
show the first lifecycle option so empty Enter works without chips; that list is owned by
the input-hints path, not by auto messages. When auto messages is off, chips and chrome
hotkeys are fully dark; empty-session under-input seed is unaffected.

Chips are action affordances (hotkey then action), not faux user messages.

## ~ Visibility

```
| Condition                                       | Chrome   |
|-------------------------------------------------|----------|
| Auto messages disabled                          | Hidden   |
| Main turn streaming                             | Hidden   |
| Composer non-empty                              | Hidden   |
| Empty chrome                                    | Hidden   |
| Auto messages on, idle, empty composer, chrome  | Shown    |
| Oneshot pending (idle + chrome + auto on)       | Shown    |
```

While a reply-suggestion oneshot is still in flight, chrome remains available when auto
messages is on so the lifecycle path does not wait on the model.

## ~ Soft hint

The first lifecycle option (when any) remains a soft hint on the reply-suggestion oneshot
request when agent input hints run. Orientation's single suggested next stage matches that
same first lifecycle option. On an empty session, that option may also seed the
under-input input-hints list (separate surface from chips).
