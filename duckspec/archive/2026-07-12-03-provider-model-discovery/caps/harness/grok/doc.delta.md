# @ Grok harness

## ~ Models

The harness discovers grok's models from the ACP handshake, which advertises the available
models, each model's display name, and each model's context window. Every model it returns
is tagged with the grok harness so it stays distinguishable once merged with other
backends' models.

Title summaries and reply-suggestion oneshots use the preferred oneshot model resolved for
the grok harness (global setting or string-match default) when that model is available.
When the preferred model is not available on the account, the harness falls back to
another available model instead of failing.
