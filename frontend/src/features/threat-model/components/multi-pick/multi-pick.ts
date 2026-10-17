import { ChangeDetectionStrategy, Component, input, model } from '@angular/core';

export interface PickOption { id: string; label: string }

/** A labelled list of checkboxes choosing ids, e.g. the assets of a component. */
@Component({
  selector: 'tm-multi-pick',
  template: `
    <fieldset class="mp">
      <legend class="field__label">{{ legend() }}</legend>
      @for (o of options(); track o.id) {
        <label class="mp__row">
          <input type="checkbox" [checked]="value().includes(o.id)" (change)="toggle(o.id)" />
          {{ o.label }}
        </label>
      } @empty {
        <p class="field__hint">{{ emptyText() }}</p>
      }
    </fieldset>
  `,
  styles: `
    .mp { border: 0; padding: 0; margin: 0 0 12px; max-height: 180px; overflow: auto; }
    .mp__row { display: flex; gap: 6px; align-items: center; font-size: var(--text-sm); padding: 2px 0; }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MultiPickComponent {
  readonly legend = input.required<string>();
  readonly options = input.required<PickOption[]>();
  readonly emptyText = input('Nothing to choose from yet.');
  readonly value = model<string[]>([]);

  toggle(id: string): void {
    const v = this.value();
    this.value.set(v.includes(id) ? v.filter(x => x !== id) : [...v, id]);
  }
}
