---
title: Design system
type: guide
status: active
tags: [ui, design, siemens-ix, frontend]
related: [docs/specification/06-ui.md, docs/guides/angular-coding-guide.md, docs/adr/0014-siemens-ix-design-system.md]
---

# Design system: Siemens Industrial Experience (iX)

The TestModeller UI uses **Siemens iX** — https://ix.siemens.io/ — as its
design system ([ADR 0014](docs/adr/0014-siemens-ix-design-system.md)).
This file is binding for every change in `frontend/`. Agents and humans
read it before writing UI code.

Versions referenced here: `@siemens/ix` 6.0.0, `@siemens/ix-angular` 6.0.0,
`@siemens/ix-icons` 4.0.0, `@siemens/ix-aggrid` 6.0.0. Token and component
names below were taken from those packages. If the iX docs and this file
disagree, the iX docs for the installed version win — fix this file in the
same change.

## Rules (must)

1. **Use an iX component when one exists.** Do not hand-build buttons,
   inputs, selects, checkboxes, toggles, modals, toasts, tabs, menus,
   dropdowns, tooltips, cards, trees, pagination, spinners or message bars.
   See [Component map](#component-map).
2. **Style only with iX tokens.** Colours, spacing, sizes, radii, borders,
   shadows and typography come from `--si-sys-*` custom properties or iX
   typography classes. No hex/rgb literals, no px font sizes, no new
   `--clr-*` tokens in component styles.
3. **Never restyle iX internals.** Do not pierce shadow DOM or override
   component internals; use the component's props, variants, slots and
   documented `--ix-*` CSS parts/variables only.
4. **Both colour schemas work.** Every screen must be usable in `dark` and
   `light` (`data-ix-color-schema`). Never assume a background colour.
5. **Icons come from `@siemens/ix-icons`** via `<ix-icon>` or the `icon`
   prop of iX buttons. No other icon sets, no inline SVG icons (the model
   canvas's diagram shapes are not icons and are exempt).
6. **Accessibility is not optional:** every input has an `ix-field-label` or
   `label`, icon-only buttons have `aria-label`, focus stays visible (iX
   focus ring, `--si-sys-color-effects-focus`), status is never shown by
   colour alone (add icon or text), WCAG 2.1 AA contrast.
7. **One primary action per view/dialog.** See [Buttons](#buttons).

Exempt from rule 2: colours the *user* picks for a node or group (stored in
the model's `layout`, so they are data, not styling), and the report's print
stylesheet, which targets paper (black on white) whatever the screen schema.

The model canvas (SVG graph editor) and threat-model diagram are custom
drawing surfaces. Their *chrome* (toolbars, palettes, context menus,
dialogs) uses iX components; the drawing itself uses iX colour tokens
(`--si-sys-color-border-*`, `--si-sys-color-data-categorical-*`,
`--si-sys-color-background-selected`, …) instead of literals.

## Setup

### Packages

```bash
npm i @siemens/ix @siemens/ix-angular @siemens/ix-icons
# only when using AG Grid:
npm i @siemens/ix-aggrid ag-grid-community
```

`@siemens/ix-angular` 6 requires Angular `>=21.2.25`.

### Global styles (`angular.json` → `styles`)

```json
"styles": ["node_modules/@siemens/ix/dist/siemens-ix/siemens-ix.css", "src/styles.scss"]
```

`siemens-ix.css` contains reset, foundation, utilities, typography and the
`community` theme (dark + light). It comes first so the app's own partials
(`styles/_tokens.scss` holds only shell geometry) can build on it. The
iX stylesheet and the shell's iX components cost about 100 kB gzipped, which
is why the initial bundle budget is 1.2 MB (warning) / 1.5 MB (error).

### Theme attributes (`index.html`)

```html
<html lang="en" data-ix-theme="community" data-ix-color-schema="system">
```

| Attribute | Values | Default for TestModeller |
| --- | --- | --- |
| `data-ix-theme` | `community` (bundled); `brand` only with the licensed Siemens brand theme package | `community` |
| `data-ix-color-schema` | `dark`, `light`, `system` | `system` |
| `data-ix-density` | `default`, `compact` | `default`; `compact` allowed for dense data views |

Switch at runtime with the `themeSwitcher` exported by `@siemens/ix`
(`setTheme`, `setColorSchema`, `toggleMode`, `getCurrentSystemAppearance`);
do not toggle CSS classes by hand.

### Angular (standalone)

Import iX components individually from the standalone entry point:

```ts
import { IxButton, IxInput, IxTextValueAccessorDirective } from '@siemens/ix-angular/standalone';

@Component({
  selector: 'app-feature-form',
  imports: [IxButton, IxInput, IxTextValueAccessorDirective, ReactiveFormsModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './feature-form.html',
})
export class FeatureForm {}
```

- Forms: use the iX value accessors (`IxTextValueAccessorDirective`,
  `IxSelectValueAccessorDirective`, `IxBooleanValueAccessorDirective`,
  `IxRadioValueAccessorDirective`, `IxDateValueAccessorDirective`, …) so
  `formControl`/`ngModel` work.
- Overlays: use the injectable `ModalService`, `ToastService`,
  `MessageService` and `LoadingService` from `@siemens/ix-angular` — never
  build modal/toast DOM yourself.
- Do not use `IxModule.forRoot()`; this app is standalone-only.
- Icons: import the constants from `@siemens/ix-icons/icons` (e.g.
  `iconSearch`) and bind them to `icon` / `name`. Named imports keep the
  bundle to the icons actually used.
- Host a component inside an iX slot with `host: { slot: '…' }` (the shell's
  `tm-topbar` and `tm-sidenav` do this for `ix-application`).
- Set initial-only props such as `ix-menu`'s `start-expanded` as static
  attributes; a property binding arrives after the component has initialised
  and is ignored.

## Foundations

### Colour (`--si-sys-color-*`)

Use semantic tokens by role, never reference tokens (`--si-ref-*`) or hex.

| Role | Tokens |
| --- | --- |
| Surfaces (page → raised) | `background-0` (app), `background-1`, `background-2`, `background-3`, `background-4` |
| Interaction states | `background-hover`, `background-active`, `background-selected`, `background-disabled` |
| Accent (primary action) | `background-accent`, `-hover`, `-active`; `text-on-accent` |
| Secondary accent | `background-accent-secondary`, `-hover`, `-active`; `text-on-accent-secondary` |
| Text | `text-primary`, `text-secondary`, `text-disabled`, `text-accent`, `text-inverse` |
| Borders | `border-1` (strongest) … `border-4` (subtlest — dividers, card outlines), `border-accent`, `border-disabled` |
| Status | `{background,text,border}-{danger,critical,warning,caution,success,information,neutral}`; subtle fills `background-*-subtle`; text on fill `text-on-*` |
| Charts / categories | `data-categorical-1` … `-17`; sequential `data-sequential-<hue>-1..4`; ratings `data-rating-{excellent,good,average,poor,bad}` |
| Code | `code-1` … `code-12` |
| Effects | `effects-focus`, `effects-backdrop`, `effects-shadow-10..40`, `effects-disable-opacity` |

All names are prefixed `--si-sys-color-`, e.g.
`background: var(--si-sys-color-background-1)`.

Domain mapping (keep consistent across screens):

| Domain meaning | Token family |
| --- | --- |
| Test passed / covered | `success` |
| Test failed / threat high | `danger` |
| Threat critical | `critical` |
| Validation warning / partially covered | `warning` |
| Needs attention, not blocking | `caution` |
| AI proposal (pending approval) | `information` |
| Not run / unknown | `neutral` |

### Typography

Font: Siemens Sans (`'SiemensSans Pro VF', 'SiemensSans Pro', helvetica, …`)
as set by the theme — do not load other fonts.

Use classes (or `<ix-typography format="…">`), not custom font rules:

| Use | Class |
| --- | --- |
| Page title | `typography-h2` (`h1` only for landing/empty states) |
| Section / panel title | `typography-h4` – `typography-h6` |
| Body, table cells, form text | `typography-body` |
| Long-form text (descriptions, AI rationale) | `typography-body-paragraph` |
| Secondary/meta text | `typography-body-sm` |
| Emphasis | `*-sbold` variants, `typography-weight-bold` |
| Code, ids, guards/actions | `typography-code`, `typography-code-sm` |
| KPI numbers | `typography-display` … `typography-display-xl` |

In SCSS, the matching shorthand tokens are `--si-sys-typography-*`
(e.g. `font: var(--si-sys-typography-body)`).

### Spacing and sizing

- Spacing: `--si-sys-sizing-spacing-x-{10..140}` (horizontal) and
  `--si-sys-sizing-spacing-y-{10..140}` (vertical). Use these for
  `padding`, `margin`, `gap`. Rem variants `--si-sys-sizing-rem-spacing-*`
  where the layout must scale with user font size.
- Sizes: `--si-sys-sizing-size-{10..200}`; icons
  `--si-sys-sizing-icon-{sm,md,lg,xl,xxl}`.
- Radius: `--si-sys-sizing-border-radius-{none,xs,sm,full}`.
- Border width: `--si-sys-sizing-border-width-{none,default,emphasis,selected}`.
- Breakpoints: `--si-sys-sizing-breakpoint-{sm,md,lg,xl,xxl}`; prefer
  `ix-layout-grid`/`ix-row`/`ix-col` or `ix-layout-auto` over hand-rolled grids.
- Density follows `data-ix-density`; do not hardcode heights for controls.

## Layout

The application shell maps onto iX like this (see `06-ui.md` → Layout):

| TestModeller area | iX |
| --- | --- |
| App root | `ix-application` |
| Top bar (product name, user) | `ix-application-header` (`tm-topbar`): route breadcrumb in `slot="secondary"`, actions as `ix-icon-button`, user menu as `ix-avatar` + `ix-dropdown-item` |
| Nav bar with Projects tree | `ix-menu` (`tm-sidenav`): the project tree in its default slot, shown only while expanded; views as `ix-menu-item`; Settings as a `slot="bottom"` item. Width via `--ix-menu--width--expanded` |
| Page body | `ix-content` + `ix-content-header` (title, back, actions) |
| Editor side & bottom panels | `ix-pane-layout` + `ix-pane` |
| Panel tabs (Test Cases / Properties, bottom tabs) | `ix-tabs` + `ix-tab-item` |
| Breadcrumb (Project > Component > Feature > Model) | `ix-breadcrumb` + `ix-breadcrumb-item` |
| Empty screens | `ix-empty-state` |
| Errors / not found | `ix-info-page` |

## Component map

| Need | Use |
| --- | --- |
| Action | `ix-button`, `ix-icon-button`, `ix-split-button`, `ix-dropdown-button`, `ix-link-button` |
| Toggle action (toolbar) | `ix-toggle-button`, `ix-icon-toggle-button` |
| Text / number / multiline | `ix-input`, `ix-number-input`, `ix-textarea` |
| Choice | `ix-select` + `ix-select-item`, `ix-radio-group`, `ix-checkbox(-group)`, `ix-toggle` |
| Dates | `ix-date-input`, `ix-datetime-input`, `ix-date-dropdown` |
| Labels & help | `ix-field-label`, `ix-helper-text` (validation via `ix-input` invalid/warning/info states) |
| Filter / search | `ix-category-filter`, `ix-filter-chip`, `ix-expanding-search` |
| Dialog | `ix-modal` (+ header/content/footer) via `ModalService` |
| Transient feedback | `ToastService` (`ix-toast`) |
| Persistent page notice | `ix-message-bar` |
| Context / overflow menu | `ix-dropdown` + `ix-dropdown-item`, `ix-group-context-menu` |
| Hint | `ix-tooltip`, `ix-popover` |
| Lists & groups | `ix-list`/`ix-list-item`, `ix-group`/`ix-group-item`, `ix-event-list` |
| Cards / tiles / KPIs | `ix-card` (+ title/content), `ix-action-card`, `ix-push-card`, `ix-kpi`, `ix-tile` |
| Key/value details (Properties) | `ix-key-value-list` + `ix-key-value` |
| Status labels | `ix-pill` (status), `ix-chip` (removable tag), `ix-badge` (count) |
| Progress | `ix-spinner`, `ix-progress-indicator`, `LoadingService` |
| Steps (generate wizard) | `ix-workflow-steps` + `ix-workflow-step` |
| Tables | `ix-pagination`; small tables: iX `table` utility classes; large/sortable: AG Grid with `@siemens/ix-aggrid` theme |
| File upload (test result import) | `ix-upload` |
| AI assistant chat | `ix-chat`, `ix-chat-input`, `ix-chat-user-message`, `ix-chat-ai-message` |
| Divider | `ix-divider` |

### Buttons

`ix-button` `variant`: `primary`, `secondary`, `tertiary`,
`subtle-primary`, `subtle-secondary`, `subtle-tertiary`,
`danger-primary`, `danger-secondary`, `danger-tertiary`.

- One `primary` per view or dialog (e.g. *Generate*, *Save*, *Accept*).
- Alternatives (`Cancel`, `Reject`) are `secondary`; low-emphasis and
  toolbar actions are `tertiary`/`subtle-*`.
- Destructive actions use `danger-*` and require a confirmation modal.
- Dialog footer order: secondary left, primary right.
- Long-running actions set `loading` instead of disabling silently.

### AI proposals (AGENTS.md rule 8)

Proposals are visibly *not yet accepted data*:
`ix-card` with an `ix-pill` "Proposal" in the `information` family,
rationale in `typography-body-paragraph`, and explicit **Accept**
(`primary`) / **Reject** (`secondary`) buttons. Never style a proposal
like accepted content.

## Content

- Sentence case for labels, buttons and titles ("Generate test cases").
- Buttons are verbs. Use the domain terms from `01-glossary.md`
  (Project > Component > Feature > Model; "Scenario" only for
  `scenarioDescription`).
- Error messages say what happened and what to do next.

## Review checklist

A frontend change is not done unless:

- [ ] iX components used wherever the component map has an entry
- [ ] No hex/rgb/px-font literals or new non-iX tokens in changed styles
- [ ] Looks right in `dark` **and** `light`
- [ ] Keyboard-operable, labelled, visible focus, status not colour-only
- [ ] At most one primary button per view/dialog
- [ ] Screenshots in `docs/screenshots/` refreshed if a screen changed

## Migration

Done:

- iX is installed and loaded globally; theme `community`, schema `system`.
- Every colour, radius, shadow, font and motion value uses iX tokens; the
  legacy `--clr-*`, `--radius-*`, `--shadow-*`, `--text-*` and
  `--transition-*` tokens are gone. Dark and light both work.
- The shell is iX: `ix-application`, `ix-application-header`,
  `ix-breadcrumb`, `ix-avatar` and `ix-menu`.

Still to do, incrementally — any component you touch moves to iX in the same
change:

- Feature pages still use the hand-built controls from `styles/_ui.scss`
  (`.btn`, tabs, dialogs, inputs, selects, pills, tables). Replace them with
  the iX components in the [Component map](#component-map), and delete each
  `_ui.scss` primitive once nothing uses it.
- Inline SVG icons in feature templates move to `@siemens/ix-icons`.
- Font sizes are rem values on the iX scale; move them to `typography-*`
  classes as templates are touched.
