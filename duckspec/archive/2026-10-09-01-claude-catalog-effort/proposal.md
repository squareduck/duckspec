# Claude catalog and effort

The Claude model menu should track the Claude Code catalog, and the composer should let a
chat set effort beside the model. Both choices apply by resuming the same Claude session
on the next turn.

Duckboard's Claude list comes from Anthropic `GET /v1/models`, then stays frozen for the
life of the process. On this machine that call cannot authenticate, so the menu falls back
to a short alias table whose newest Sonnet label is 4.6. The `claude` CLI shows Sonnet 5.5
from a different roster: surface `cc` of
`https://downloads.claude.ai/model-catalog/v1/catalog.json`. That catalog already carries
the current models, their context windows, and a per-model effort scale. The public models
API is a different roster, so repairing its auth would still leave the menu wrong.

The picker offers only catalog rows in `section: main` that the installed `claude` meets
`min_claude_code_version` for. Display names and context windows come from those rows, and
the selected id is what `--model` receives. Refresh when the app starts and again after
the catalog's `expires_at`. A failed refresh keeps the last good Claude slice.

Effort sits in the composer footer beside the model selector, on Claude chats only. The
control lists the levels that model's catalog entry offers. Until the chat overrides it,
the shown level is the catalog default. A model with no effort omits the control.

```
| Chat state        | Effort used                              |
| ----------------- | ---------------------------------------- |
| No override       | Catalog default for the selected model   |
| User set a level  | That level, kept on the chat across turns |
| User changes model | The new model's catalog default         |
```

A model pick today only stores the choice on the chat. A hot Claude process keeps the
`--model` it was spawned with. Model and effort share one apply rule: when that process
does not match the chat's model or effort, the next turn ends it and resumes the same
Claude session with `--model` and `--effort`.

```
chat choice          hot process
model + effort       --model + --effort
       │
       ▼  mismatch
 end process, resume the same session
 with the chat's flags
```

Grok and Codex stay without this control. Overflow catalog rows stay out of the menu.
Oneshot title and reply model selection stays as it is.
