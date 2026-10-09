# @ Chat composer footer

The meta strip under the chat prompt: session and attachment hints, a short model control,
an effort control when the selected model has one, and a progressive context-usage
readout. Visual chrome (paper blend, lightweight controls) is presentation detail; this
capability owns the honest behavioral rules.

Until a chat pins a level, the effort control shows that model's catalog default. A main
send uses the pinned level, or that default when the chat has no pin.

## + Effort

The effort control is the pick list after the model control and before the usage readout.
It is shown when the effective model is available and that model's catalog row has an
effort scale. It is hidden when the model is Missing or the row has no effort.

With no pin, including a chat saved before effort existed, the control shows the row's
catalog default. Choosing any offered level, including that default, stores a pin on the
chat. A later change to the catalog default leaves a pin in place.

```
| id     | Closed label |
| ------ | ------------ |
| low    | Low          |
| medium | Medium       |
| high   | High         |
| xhigh  | Xhigh        |
| max    | Max          |
```

Any other id is shown unchanged. The menu lists the row's levels in catalog order and has
no Default row.

When session defaults are stamped, the pin is reconciled with the preferred model. No
preferred model keeps the pin. The same preferred model keeps the pin while the level is
still offered. A different preferred model clears the pin, and so does a level that the
available row no longer offers. The clear is saved. If the same model drops out of the
catalog and later returns, the pin is still there, and the control stays hidden while the
model is Missing.

A main-chat send uses the pinned level, or the catalog default when the chat has no pin. A
row with no effort scale sends no effort.
