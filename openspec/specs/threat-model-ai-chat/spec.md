# threat-model-ai-chat Specification

## Purpose
The chat panel in the threat model workspace through which the user talks to the AI assistant and reviews its proposals.

## Requirements

### Requirement: Chat panel
The workspace SHALL offer an AI chat window, as the model editor does: a launcher in the bottom-right corner opens a window that floats over the page, can be resized, minimised and closed, and is available on every tab. A message SHALL produce one request; the panel SHALL show a pending indicator, then one card per resulting proposal. The transcript SHALL exist only in the browser and no earlier message SHALL be sent with a new request.

#### Scenario: Ask
- **WHEN** the user types "what could go wrong with the payment API?" and sends
- **THEN** the panel shows the message, a pending indicator and then threat cards

#### Scenario: Close and reopen
- **WHEN** the user closes the panel and reopens it in the same session
- **THEN** the transcript is still shown

#### Scenario: Reload
- **WHEN** the page is reloaded
- **THEN** the transcript is empty and pending proposals are still available in the proposals list

### Requirement: Kind selection and focus
The panel SHALL infer the proposal kind from the message and SHALL let the user override it with a selector. When an entity is selected in a table or on the diagram, the panel SHALL offer it as the focus of the request ("about THR-0004"), which becomes the target ids of the request.

#### Scenario: Inferred kind
- **WHEN** the user writes "suggest controls for THR-0004"
- **THEN** the request kind is `tm-controls` with THR-0004 as target

#### Scenario: Override
- **WHEN** the user picks "Ratings" in the selector
- **THEN** the request uses `tm-ratings` regardless of the text

### Requirement: Cards
Each card SHALL show the proposed content, the rationale and the source model, and the actions Accept, Edit and Reject. Accepting SHALL call the backend first and update the workspace only after success. Review-kind answers SHALL render as text with no accept action. Errors (no provider, AI disabled, provider failure, invalid output) SHALL appear in the transcript with what to do.

#### Scenario: Accept card
- **WHEN** the user accepts a control card
- **THEN** the card shows "Accepted" with a link to the new control and the Controls tab includes it

#### Scenario: Review answer
- **WHEN** the user asks "what is missing in this model?"
- **THEN** the answer is shown as text findings with links to the entities mentioned and no accept action

#### Scenario: Provider error
- **WHEN** the provider is unreachable
- **THEN** the transcript shows the error and suggests checking the AI settings

### Requirement: Disabled state
When the provider is `none` or AI assistance is disabled for the threat model, the panel SHALL explain that and offer the way to enable it instead of an input field.

#### Scenario: Disabled
- **WHEN** the user opens the panel on a threat model with AI disabled
- **THEN** it explains why and offers the enable switch to users allowed to change the threat model
