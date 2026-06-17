# Validated merge wrappers

Add the validated merge entry points over `apply_delta` in `duckpond::merge`: one generic
result type, one error type, and the two kind-specific wrappers.

## Tasks

- [x] 1. In `duckpond::merge`, add `Merged<A>` (`Updated { rendered, artifact }` /
         `Deleted`), `MergeValidateError` (`Merge(Vec<MergeError>)` /
         `Parse(Vec<ParseError>)`), and `summarize_errors`; implement `merge_spec_delta`
         and `merge_doc_delta` as `apply_delta` plus a re-parse with `parse_spec` /
         `parse_document` respectively. Leave `apply_delta` unchanged

- [x] 2. @spec merge/validate Validated merge outcome: A successful spec merge returns the rendered markdown and the parsed spec

- [x] 3. @spec merge/validate Validated merge outcome: A delta that deletes the artifact yields a deletion outcome

- [x] 4. @spec merge/validate Validated merge outcome: A doc merge is validated with the document parser

- [x] 5. @spec merge/validate Failure classification: A delta that does not apply returns a merge error

- [x] 6. @spec merge/validate Failure classification: Merged text that violates its schema returns a parse error

- [x] 7. @spec merge/validate Failure classification: A multi-error failure renders as one summarized line
