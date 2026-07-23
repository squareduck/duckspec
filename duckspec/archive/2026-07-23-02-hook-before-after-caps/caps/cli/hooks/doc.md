# Stage hooks

Project-local markdown under `duckspec/hooks/` injects custom instructions into stock
stage templates without forking the templates themselves.

## Files and names

Each hook is one optional file:

```
duckspec/hooks/<stage>-before.md
duckspec/hooks/<stage>-after.md
```

`<stage>` matches a stock agent stage (`explore`, `propose`, `apply`, and the rest of the
known set). Position tokens are only `before` and `after` — the same words used in create
flags and in the template section headers.

The `hooks/` directory is optional. `ds init` does not create it; create makes the
directory when scaffolding the first file.

## Create and template

```
ds create hook <stage> --before|--after
        │
        ▼
hooks/<stage>-before|after.md   (H1 seed only)
        │
ds template <stage>
        │
        ▼
## Before write / ## After write  (only when body is non-empty)
```

```
| Command | Role |
| --- | --- |
| `ds create hook` | Scaffold a new file; gate stage name and refuse overwrite |
| `ds template` | Load hooks for the stage name and inject into the stock template |
```

Create validates the stage against the known scaffolding list. Template does not re-check
that list: it prints a stock template by name and loads hooks if present. Unknown template
names fail because the stock template is missing, not because of the hook stage gate.

## Body shape

Hook content is freeform markdown. Injection does not require an H1 and does not strip
one. Create seeds `# <Stage> - Before` or `# <Stage> - After` so editors and agents have a
non-empty file to open; authors may replace or delete that heading.

Empty or whitespace-only files count as absent: the matching `## Before write` or
`## After write` placeholder is dropped from the rendered template. Non-empty bodies are
emitted trimmed, under the section header, verbatim.

## Known stages (create)

Create accepts only:

`explore`, `backfill`, `propose`, `design`, `spec`, `step`, `apply`, `archive`, `verify`,
`review`, `followup`, `codex`.

Help text for `ds create hook` lists the same set. Adding a stage means extending that
shared scaffolding list and shipping a stock template with the usual placeholders.
