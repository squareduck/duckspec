# @ Claude harness

## ~ Duplex heat

After the first prompt has started Claude, the agent keeps a long-lived `claude` duplex
session (`--input-format stream-json` and `--output-format stream-json`) for the main path
while that process is still the session and was spawned with the prompt's model and
effort. A matching prompt reuses the process. A prompt whose model or effort differs ends
it and starts Claude again with `--resume` for the same native session id, `--model`, and
`--effort` when the prompt has a level. Cancel ends heat; the next turn may start Claude
again and still resume a prior native session id. Changing model or effort during an
in-flight turn waits for the following prompt. Title and reply oneshots do not use this
process.

## ~ Model discovery

The host does not fetch the catalog and does not keep a static Claude model table. During
initialize the owned agent fetches the public Claude Code catalog
(`https://downloads.claude.ai/model-catalog/v1/catalog.json`) with no auth and advertises
the rows a reader can select.

```
catalog document
   │  main section, and minimum version absent or met by this claude binary
   ▼
initialize advertise set
   │  name, context window, effort scale when the row has one
   ▼
host model list
```

The binary version is the leading `N.N.N` from `claude --version`, compared
component-wise. When that version cannot be read, only rows with no minimum stay. A row's
effort scale is its levels in catalog order plus the default level, and only when the row
defines one. Initialize also says whether the fetch succeeded, and includes the document's
expiry when the document has one. A failed fetch still finishes initialize and advertises
no models.

The host lists whatever that initialize result advertised. Keeping a prior Claude slice
when a later fetch fails belongs to the harness model catalog.

## @ Process tree

```
duckboard / duckchat worker
   │  ACP (shared client) — mid-turn user choice when Claude asks
   ▼
duckchat-claude-acp          (owned agent)
   │  stream-json duplex + control / canUseTool
   ▼
claude                       (official CLI)
```

Selecting the Claude harness only changes the provider launch (the agent binary). Turn
lifecycle, event mapping, and main heat for the **agent** process are the shared ACP
client. This capability owns Claude-specific behavior: agent binary discovery, when the
inner `claude` process starts, Claude-native session ids after the first prompt, duplex
heat of that process while the prompt's model and effort still match, catalog
advertisement on initialize, translating Claude's stream into the client's dialect
profile, and bridging AskUserQuestion to the parent's user-choice loop.
