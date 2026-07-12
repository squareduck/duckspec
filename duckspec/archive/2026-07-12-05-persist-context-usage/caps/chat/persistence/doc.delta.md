# @ Chat persistence

## + Last-known context usage

The durable session carries a last-known context usage total — the token count that drives
the composer's context meter. After a successful save and reload, that total is the same
as before the save. Session files written before this field existed still load; missing
usage is treated as zero.

Usage is last-known from the agent harness, not estimated from transcript size. The meter
window (denominator) is not stored on the session; it comes from the selected model when
known.
