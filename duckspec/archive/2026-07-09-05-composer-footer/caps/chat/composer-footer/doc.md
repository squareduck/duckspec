# Chat composer footer

The meta strip under the chat prompt: session and attachment hints, a short model control,
and a progressive context-usage readout. Visual chrome (paper blend, lightweight controls)
is presentation detail; this capability owns the honest behavioral rules.

## Resend-history hint

When the next send cannot resume the agent session, duckboard may re-send prior turns as a
history preamble. The footer calls that case out only when it would actually happen:

```
| Next send resumes? | Transcript | Hint        |
|--------------------|------------|-------------|
| yes                | any        | hidden      |
| no                 | empty      | hidden      |
| no                 | non-empty  | shown       |
```

Empty chats that start fresh show no resend warning — there is nothing to resend.

## Progressive usage readout

Fill is still measured against the selected model's context window (see the harness model
picker). The footer only chooses how densely to show a known fill:

```
| Fill vs window | Readout form              |
|----------------|---------------------------|
| known, < 75%   | percentage only           |
| known, ≥ 75%   | used / max and percentage |
```

The 75% threshold matches the existing warning color band for high context pressure. An
unknown window still yields no fill (owned by the model-picker meter rules).

## Closed model label

The closed control shows the model's short display name only (for example `Grok 4.5`).
Harness grouping remains how the open menu lists choices so backends stay distinguishable;
it is not repeated in the closed label.
