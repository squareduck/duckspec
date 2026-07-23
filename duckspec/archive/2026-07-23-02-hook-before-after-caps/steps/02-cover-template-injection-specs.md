# Cover template injection specs

Attach `@spec` backlinks to the existing template injection behavior (and fill any thin
coverage) so the three injection scenarios resolve.

## Tasks

- [x] 1. Align `template.rs` unit tests with the three injection scenarios (including
         missing **or** empty as one story)

- [x] 2. @spec cli/hooks Template injection: Non-empty hooks inject under section headers

- [x] 3. @spec cli/hooks Template injection: Missing or empty hooks drop the placeholders

- [x] 4. @spec cli/hooks Template injection: Body without H1 is rendered verbatim
