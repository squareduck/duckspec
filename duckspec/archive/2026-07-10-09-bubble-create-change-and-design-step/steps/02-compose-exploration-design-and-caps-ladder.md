# Compose exploration, design, and caps ladder

Wire pre-step composition (exploration Create change; design and caps rungs without
reviews) and cover those scenarios.

## Prerequisites

- [x] @step add-affirm-createchange

## Tasks

- [x] 1. Nonempty exploration: affirm `CreateChange`, empty lifecycle, no decline; empty
         exploration stays `/ds-explore` only

- [x] 2. Design, no caps, no reviews: lifecycle `&["ds-spec", "ds-step"]`

- [x] 3. Caps, no steps, no reviews: lifecycle `&["ds-step", "ds-archive"]`

- [x] 4. @spec chat/obvious-bubble Chrome composition: Nonempty exploration yields Create change only

- [x] 5. @spec chat/obvious-bubble Chrome composition: Design without caps yields spec then step

- [x] 6. @spec chat/obvious-bubble Chrome composition: Caps without steps yield step then archive
