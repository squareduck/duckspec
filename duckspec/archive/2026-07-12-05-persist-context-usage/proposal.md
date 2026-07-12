# Persist context usage

Remember the last known context fill for each chat session so the composer meter stays
honest after an app restart, instead of showing 0% until the next turn.

## Motivation

After restart, existing chats show 0% context until a new message produces a usage update.
The transcript, model choice, and agent session resume already survive; only the usage
numerator is dropped. That makes long chats look empty on open even when they were nearly
full moments earlier.

Why now: the meter is already trusted mid-session; a one-field durability gap is the only
reason restart lies about fill.

## Intent

- Last known context usage for a session is durable across app restart

- Opening an existing session shows approximately the fill last observed during a turn,
  without requiring a new message first

- Fresh sessions and sessions that never received usage still read as empty / zero until
  real usage arrives

- Legacy session files without a stored value load cleanly (treat as unknown / zero)

- The meter remains "last known," not a live probe of the agent process while idle

## Non-goals

- Estimating usage from transcript size when the harness never reported tokens

- Showing an explicit "stale / last known" label in the footer

- Persisting or fixing context window lookup beyond what the selected model already
  provides

- Changing progressive readout rules (cool vs hot fill formatting)

- Recalculating usage on model switch without a new harness report
