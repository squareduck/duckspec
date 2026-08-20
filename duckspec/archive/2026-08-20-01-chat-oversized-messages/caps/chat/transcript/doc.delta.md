# @ Chat transcript

## ~ Secondary chrome

User messages keep a paper card and Answer prose stays primary on the chat background. An
oversized user body is shown as a source prefix on that card; the full body stays on the
session (`chat/oversized-messages`). Thinking and Activity share one secondary
presentation: flat collapsible header (chevron and muted label) and no group-level card
chrome. Labels alone disambiguate the two kinds (`Thinking · N lines` vs `N tools · …`).

```
| Segment  | Chrome                                      | Role            |
| -------- | ------------------------------------------- | --------------- |
| User     | paper card (prefix if oversized)            | primary object  |
| Answer   | plain / last-answer band                    | primary reply   |
| Thinking | flat header (muted label)                   | secondary (why) |
| Activity | flat header + quiet tool rows               | tertiary (what) |
```
