# CLAUDE.md

@AGENTS.md

## UI work

Any change in `frontend/` must follow @DESIGN.md (Siemens iX design system,
https://ix.siemens.io/). Read it before writing UI code. In short:

- Use an iX component (`@siemens/ix-angular/standalone`) wherever one exists;
  do not hand-build controls, modals, toasts, menus or tabs.
- Style only with `--si-sys-*` tokens and `typography-*` classes: no hex/rgb
  colours, px font sizes or new custom tokens.
- Icons from `@siemens/ix-icons`; overlays via `ModalService` / `ToastService`.
- Works in both `dark` and `light` colour schemas; labelled, keyboard-operable,
  status never colour-only; one primary button per view.
- Run the review checklist at the end of `DESIGN.md` before calling it done.
