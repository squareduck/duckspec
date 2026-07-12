# @ Chat composer footer

## ~ Resend-history hint

When a stored agent session id cannot resume on the effective harness (for example after a
harness switch), the next send may re-feed prior turns as a history preamble. The footer
calls that case out only when it is durable and user-relevant:

```
| Transcript | Stored agent session id | Resumable for harness? | Hint   |
|------------|-------------------------|------------------------|--------|
| empty      | any                     | any                    | hidden |
| non-empty  | none                    | —                      | hidden |
| non-empty  | present                 | yes                    | hidden |
| non-empty  | present                 | no                     | shown  |
```

No stored id covers first bind and post-recovery clear — the footer stays silent even if a
later send still re-feeds history.
