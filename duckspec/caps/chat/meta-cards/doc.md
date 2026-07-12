# Chat meta cards

Duckboard-local recognition of chat `write` and `next` meta cards in assistant markdown:
which quote runs count as cards, their line ranges, and which send tokens a trailing
`next` card contributes.

## What a meta card is

In chat, a meta card is chat chrome expressed as a markdown blockquote whose first
non-empty content line is exactly `**write**` or `**next**` (after trim). Other
blockquotes are ordinary prose quotes and are ignored by this capability.

```
> **next**
>
> `confirm proposal`
> `/ds-spec`  write specs
> `reject proposal`
```

The kind line names the card. Body lines are consecutive blockquote lines until the first
non-blockquote line ends the run. Decision tokens stand alone; slash-command lines may
carry a short reason after the token (reason is not part of the send text).

## Recognition

```
| Input shape                                              | Result                          |
|----------------------------------------------------------|---------------------------------|
| `>` run, first non-empty content `**write**` / `**next**`| One card of that kind           |
| `>` run, first non-empty content anything else           | Not a meta card                 |
| `> **next**` only inside a fenced code block             | Not a meta card                 |
```

Each card reports an inclusive 0-based line range covering the whole quote run (kind line
through last body line of the run). Those ranges are the shared input for transcript
tinting and for locating a trailing `next` card.

Fence tracking is local to this scanner: while a fenced code block is open, lines are not
treated as blockquote lines even if they start with `>`.

## Trailing next actions

Only a **trailing** `next` card supplies empty-composer actions: the card must end at the
last non-empty line of the message (blank lines after it are fine). A `next` card
mid-message, or a trailing `write` card alone, yields no actions.

```
| Body line shape                         | Contribution                         |
|-----------------------------------------|--------------------------------------|
| First `` `token` `` present             | One action; send text = that token   |
| Text after the first code span          | Reason only — not part of send text  |
| No inline code span                     | Skipped                              |
| Fourth and later token-bearing lines    | Ignored (cap 3)                      |
```

Actions keep source order. Send text is exactly the token inside the first pair of single
backticks on the line (for example `confirm proposal`, `/ds-propose`).
