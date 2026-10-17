import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import {
  ArrowKind, ElementStyle, LineDash, LineWidth, STYLE_KEYS, STYLE_KEYS_FOR, StyleKey, StyleValue, TextSize,
} from '../../state/model-editor.store';
import { ColorControlComponent, MIXED, Mixed } from './color-control';

/** One element being styled: what kind it is and its current style. */
export interface StyleTarget { kind: keyof typeof STYLE_KEYS_FOR; style?: ElementStyle }

/** A change the user made: one key, for every target that supports it. */
export interface StyleChange { key: StyleKey; value: StyleValue }

/**
 * The style controls of the Properties panel, for one element or several.
 *
 * Shows only the keys every target supports, a mixed state where their
 * values differ, and emits each change; the host applies it to all of them.
 */
@Component({
  selector: 'tm-style-section',
  imports: [ColorControlComponent],
  templateUrl: './style-section.html',
  styleUrl: './style-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StyleSectionComponent {
  readonly targets = input.required<StyleTarget[]>();
  readonly changed = output<StyleChange>();

  readonly dashes: { value: LineDash | undefined; label: string; dash: string | null }[] = [
    { value: undefined, label: 'Solid', dash: null },
    { value: 'dashed',  label: 'Dashed', dash: '5 3' },
    { value: 'dotted',  label: 'Dotted', dash: '0 3.5' },
  ];
  readonly widths: { value: LineWidth | undefined; label: string }[] = [
    { value: undefined, label: 'Auto' }, { value: 1, label: '1' }, { value: 2, label: '2' },
    { value: 3, label: '3' }, { value: 4, label: '4' },
  ];
  readonly arrows: { value: ArrowKind | undefined; label: string }[] = [
    { value: undefined, label: 'Filled' }, { value: 'open', label: 'Open' }, { value: 'line', label: 'Line' },
  ];
  readonly sizes: { value: TextSize | undefined; label: string; name: string }[] = [
    { value: 's', label: 'S', name: 'Small' }, { value: undefined, label: 'M', name: 'Normal' },
    { value: 'l', label: 'L', name: 'Large' },
  ];

  /** Keys every target supports, in display order. */
  readonly keys = computed(() => {
    const ts = this.targets();
    if (!ts.length) return new Set<StyleKey>();
    return new Set(STYLE_KEYS.filter(k => ts.every(t => STYLE_KEYS_FOR[t.kind].includes(k))));
  });

  has(key: StyleKey): boolean { return this.keys().has(key); }
  hasLine(): boolean { return this.has('stroke') || this.has('dash') || this.has('width'); }
  hasText(): boolean { return this.has('text') || this.has('size') || this.has('bold') || this.has('italic'); }

  /** The targets' common value of a key (undefined is the default), or "mixed". */
  value<K extends StyleKey>(key: K): ElementStyle[K] | Mixed {
    const vals = this.targets().map(t => t.style?.[key]);
    return vals.every(v => v === vals[0]) ? vals[0] : MIXED;
  }

  colour(key: 'stroke' | 'fill' | 'text'): string | null | Mixed {
    return this.value(key) ?? null;
  }

  /** aria-checked of an option: true, false, or "mixed" when the targets differ. */
  checked(key: StyleKey, option: unknown): 'true' | 'false' | 'mixed' {
    const v = this.value(key);
    if (v === MIXED) return 'mixed';
    return v === option ? 'true' : 'false';
  }

  /** aria-pressed of a flag such as bold. */
  pressed(key: 'bold' | 'italic'): 'true' | 'false' | 'mixed' {
    const v = this.value(key);
    return v === MIXED ? 'mixed' : v ? 'true' : 'false';
  }

  set(key: StyleKey, value: StyleValue | undefined): void {
    this.changed.emit({ key, value: value ?? null });
  }

  /** A flag turns on unless every target already has it. */
  toggle(key: 'bold' | 'italic'): void {
    this.set(key, this.value(key) === true ? null : true);
  }
}
