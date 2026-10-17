import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

/**
 * Marks an entity that came from an accepted AI proposal, with the model id
 * as its tooltip. Text first, so it reads without colour.
 */
@Component({
  selector: 'tm-ai-badge',
  template: `@if (origin() === 'ai') {
    <span class="ai-badge" [attr.title]="tip()">✦ AI</span>
  }`,
  styles: `.ai-badge { border: 1px solid currentColor; border-radius: 4px; padding: 0 6px; font-size: .75rem; margin-left: 6px; }`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AiBadgeComponent {
  readonly origin = input<string | null | undefined>(null);
  readonly source = input<string | null | undefined>(null);
  readonly tip = computed(() => `Proposed by ${this.source() || 'the AI assistant'}; accepted by an analyst`);
}
