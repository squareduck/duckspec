# Title slug

The single rule that turns a human-written title into a kebab-case slug, used wherever
duckspec derives a filename or identifier from a title.

## The rule

Slugification is a pure transformation from a title string to a slug string, in four
steps:

1. Lowercase the whole title.

2. Keep alphanumeric characters. This is Unicode-aware — letters and digits in any script
   are preserved, not just ASCII.

3. Map every run of one or more non-alphanumeric characters to a single `-`.

4. Trim any leading or trailing `-`.

The result is a string of lowercase alphanumeric tokens joined by single dashes, with no
leading, trailing, or doubled dashes.

```
"Post-implementation: soundness & fidelity"  →  "post-implementation-soundness-fidelity"
"  Set  up   database  "                      →  "set-up-database"
"Café Résumé"                                  →  "café-résumé"
```

## Empty results

A title with no alphanumeric characters — all punctuation, symbols, or whitespace —
slugifies to the empty string. The rule does not substitute a placeholder or raise an
error; it returns `""` and leaves the decision to the caller.

This keeps the rule usable in every context: a caller that needs a non-empty filename
rejects the empty result, while a caller that can tolerate a fallback substitutes its own
default. The transformation itself stays free of any single caller's policy.
