import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { NODE_COLORS } from '../../state/model-editor.store';

/** "Mixed": the selected elements have different values. */
export const MIXED = 'mixed' as const;
export type Mixed = typeof MIXED;

/**
 * A colour choice: "Default", the preset palette, and "Custom" (the browser's
 * colour picker). A colour outside the palette shows as the custom value.
 */
@Component({
  selector: 'tm-color-control',
  templateUrl: './color-control.html',
  styleUrl: './color-control.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ColorControlComponent {
  /** What the control is for, e.g. "Line colour"; names the group and its swatches. */
  readonly label = input.required<string>();
  /** Current colour, null for the default, or "mixed". */
  readonly value = input<string | null | Mixed>(null);
  readonly pick = output<string | null>();

  readonly presets = NODE_COLORS.filter(c => c.value !== null) as { name: string; value: string }[];

  readonly isMixed = computed(() => this.value() === MIXED);
  /** A chosen colour that is not one of the presets. */
  readonly custom = computed(() => {
    const v = this.value();
    if (!v || v === MIXED) return null;
    return this.presets.some(p => p.value.toLowerCase() === v.toLowerCase()) ? null : v;
  });

  isOn(v: string | null): boolean {
    const cur = this.value();
    if (cur === MIXED) return false;
    if (cur === null || v === null) return cur === v;
    return cur.toLowerCase() === v.toLowerCase();
  }

  /** Value for the native picker: the custom colour, else the current one, else black. */
  pickerValue(): string {
    const v = this.value();
    return v && v !== MIXED && /^#[0-9a-f]{6}$/i.test(v) ? v : '#000000';
  }

  onCustom(e: Event): void {
    this.pick.emit((e.target as HTMLInputElement).value);
  }
}
