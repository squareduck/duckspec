# Exploration promotion

An exploration is a free-form chat scope that may become a real change. Promotion is the
moment that transition is recognized and the exploration's chat is migrated to the
change's scope. This capability governs *when* that happens and *which* exploration is
chosen.

## The attribution problem

Promotion is driven by detecting a change directory that is present now but was not in
duckboard's previous view of the project. That detection alone is ambiguous: a directory
can become newly-present for reasons that have nothing to do with an exploration
finishing.

```
directory detected as newly present
├── an exploration's agent just ran `ds create change` → genuine promotion
├── a change was created out-of-band (CLI, another tool)
├── a version-control operation restored the working copy (checkout, undo)
└── a change was unarchived
```

Only the first case is a promotion. Choosing an exploration for any of the others — in
particular by picking whichever exploration the user is currently looking at — attributes
an unrelated chat to the change and, once the link is written, is sticky. The policy below
removes that guess.

## Bindings

A binding is the authoritative record that a specific exploration created a specific
change. It is established at the causal moment: when an exploration session's agent runs
`ds create change <name>`, duckboard records `<name> → <exploration>`. Promotion consults
only bindings.

```
| Situation | Binding? | Outcome |
| --- | --- | --- |
| Exploration's agent ran `ds create change foo` | yes | `foo` adopts that exploration |
| Change `foo` created out-of-band | no | `foo` left standalone |
| Working-copy restored `foo`'s directory | no | `foo` left standalone |
| `foo` unarchived | no | `foo` left standalone |
```

When a change is left standalone, nothing is lost: any exploration the user was viewing
keeps its own chat scope, and the change simply has no adopted chat.

## Single-use bindings

A binding is consumed by the promotion it authorizes. Because directory detection can fire
again for the same change — a version-control reappearance after the change already exists
— a consumed binding must not promote a second time. After consumption there is no
binding, so the reappearance is treated like any other unbound detection and no promotion
occurs.

## Trade-off

Requiring a binding means a change created entirely outside an exploration's agent (for
example, typed directly into a terminal) will not automatically pull that terminal's
exploration chat into the change. This is a deliberate, minor loss of convenience in
exchange for never misattributing an unrelated chat. The chat remains available under its
original scope and can be referenced there.
