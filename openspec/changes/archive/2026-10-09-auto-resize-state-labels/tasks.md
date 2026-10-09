# Tasks

## 1. Sizing logic

- [x] 1.1 Add a pure `fitNodeSize(label, shape)` with word wrapping and min/max bounds in the model store layer; verify unit tests cover short, long, unbreakable-long and empty labels for rect, circle and diamond
- [x] 1.2 Call it from `updateNode` on label/shape change (resizing around the centre) and when a model loads; verify store tests show size grows, shrinks back to the default minimum, and the centre stays fixed

## 2. Canvas rendering

- [x] 2.1 Render multi-line labels as `<tspan>` lines vertically centred, and size the inline edit box to the node; verify a canvas test shows a long name stays within the node bounds
- [x] 2.2 Re-fit after web fonts load; verify no overflow after `document.fonts.ready`
- [x] 2.3 Verify transitions, connector dots, test chips and groups follow a resized state (canvas spec)

## 3. Docs and checks

- [x] 3.1 Add the sizing rule to `docs/specification/06-ui.md`; confirm against `openapi.yaml` that `w`/`h` are already persisted so no API change is needed
- [x] 3.2 Run `ng lint` and frontend tests; verify both pass
