# Embed CLI content

Ship the stock command, template, and schema files inside the `ds` binary so release
installs work without the source tree that built them.

## Motivation

`ds init <harness>`, `ds template`, and `ds schema` read stock files from a path baked in
at compile time (`CARGO_MANIFEST_DIR`). That works on a machine where the checkout still
sits at the build path (typical local Mac installs). It fails with a bare "No such file or
directory" for release tarballs and any other install that only has the binary — observed
on Linux with the published artifact.

Docs already describe this content as embedded; the CLI does not yet do that. Until it
does, the advertised install path is broken for everyone who follows it.

## Intent

- A standalone `ds` binary carries everything needed for `init` (harness command install),
  `template`, and `schema` — no runtime dependency on the build machine's filesystem

- Behavior of those commands stays the same when content is present: install the same
  files, print the same text

- Failure modes for unknown harness / template / schema names stay clear; they are not
  confused with missing on-disk source trees

## Non-goals

- Changing harness command content or which harnesses `init` supports

- Project-local overrides of stock templates/schemas (hooks already cover project-specific
  template injection)

- Packaging or shipping the `content/` tree next to the binary as a separate asset bundle

- Changes to duckboard or duckpond
