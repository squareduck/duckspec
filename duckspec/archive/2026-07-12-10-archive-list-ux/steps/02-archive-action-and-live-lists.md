# Archive action and live lists

Wire soft archive into live exploration lists and the Change-list hover control (archive
when live, remove when archived).

## Prerequisites

- [x] @step exploration-soft-archive-model

## Tasks

- [x] 1. Filter Change picker and Dashboard Explorations to non–idea-owned and
         non-archived explorations

- [x] 2. Add `ArchiveExploration` in `area/change.rs`: set stamp, save explorations, clear
         arm state; do not delete chats

- [x] 3. Hover leading control: live → archive (one click); archived → existing remove
         arm/commit path

- [x] 4. @spec exploration/archive Live list membership: Archived non–idea-owned exploration is absent from live lists

- [x] 5. @spec exploration/archive Live list membership: Live non–idea-owned exploration remains on live lists

- [x] 6. @spec exploration/archive Hover control by state: Live exploration hover control archives

- [x] 7. @spec exploration/archive Hover control by state: Archived exploration hover control removes

- [x] 8. @spec exploration/archive Hover control by state: Remove with sessions requires arm then commit

- [x] 9. @spec exploration/archive Hover control by state: Remove with no sessions commits without arm
