# @ Session scope orientation

## ~ Lifecycle and next stage

The next stage is derived from which artifacts the change has and how far its steps have
progressed:

```
change state                                  suggested next stage
────────────────────────────────────────────  ────────────────────
proposal only                                  design
design, no specs                               spec
specs, no steps                                step
steps with at least one incomplete             apply
all steps complete                             archive
```
