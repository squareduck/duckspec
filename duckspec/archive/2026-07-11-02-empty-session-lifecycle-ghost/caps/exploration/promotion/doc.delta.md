# @ Exploration promotion

## @ Bindings

A binding is the authoritative record that a specific exploration created a specific
change. It is established at the causal moment: when an exploration session's agent runs
`ds create change <name>`, duckboard records `<name> → <exploration>`. Promotion consults
only bindings.

```
| Situation | Binding? | Outcome |
| --- | --- | --- |
| Exploration's agent ran `ds create change foo` | yes | `foo` adopts that exploration; chat input focused |
| Change `foo` created out-of-band | no | `foo` left standalone; no forced chat focus |
| Working-copy restored `foo`'s directory | no | `foo` left standalone; no forced chat focus |
| `foo` unarchived | no | `foo` left standalone; no forced chat focus |
```

When a change is left standalone, nothing is lost: any exploration the user was viewing
keeps its own chat scope, and the change simply has no adopted chat.

## + Chat focus after promotion

When a binding authorizes promotion, the exploration's chat migrates to the change scope
and the chat input is given keyboard focus again. Scope migration remounts chat UI;
without an explicit refocus the user would have to click the input to keep typing. Unbound
new directories never promote and do not force focus onto the chat input — focus behavior
for those detections stays as it was.
