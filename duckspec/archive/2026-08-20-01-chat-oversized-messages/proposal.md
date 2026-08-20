# Chat oversized messages

Transcript User, Answer, and Thinking bodies past a size gate render as an unhighlighted
prefix with a control to copy the stored full text. Storage and the agent keep the
original message.

Pasting a large JSON file into chat currently freezes duckboard. Chat editors assume every
message is small: they run full markdown syntax highlighting on the UI thread and grow to
the full height of the body. File tabs already highlight off-thread; Activity already caps
tool dumps at 10 lines. User, Answer, and Thinking do not.

The size gate has to catch both a wall of lines and a single minified JSON line — line
count alone would miss the latter. Past the gate:

- the transcript editor shows a prefix of that size, with no syntax highlight
- a control copies the stored full body (ordinary selection still copies the prefix)
- there is no expand-in-place, which would put the giant widget back on screen

Truncation is display-only. The session message stays whole, so persistence, resume, and
the agent are unchanged. Already-saved sessions (the original freeze) get the same
treatment. A live answer that grows past the gate starts truncating as it streams.

Out of scope: virtualizing the transcript, async highlight for ordinary-sized messages,
changing Activity’s 10-line tool dump, treating the paste as a file attachment, mutating
stored chat. The composer already caps height at 20 rows; its remaining sync-highlight of
a huge paste is adjacent, not this change.
