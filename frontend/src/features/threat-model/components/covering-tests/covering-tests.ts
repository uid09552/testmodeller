import {
  ChangeDetectionStrategy, Component, effect, inject, input, signal, untracked,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import { IxPill } from '@siemens/ix-angular/standalone';
import { TestCase } from '../../../../core/api/api.types';
import { OrgApi } from '../../../../core/api/org-api';

/** How a covering test case's latest result reads; status is never colour only. */
export function resultBadge(tc: TestCase): { text: string; variant: 'success' | 'danger' | 'neutral' } {
  switch (tc.lastResult?.status) {
    case 'passed': return { text: '✓ Passed', variant: 'success' };
    case 'failed':
    case 'error': return { text: '✗ Failed', variant: 'danger' };
    case 'skipped': return { text: '– Skipped', variant: 'neutral' };
    default: return { text: '– No result', variant: 'neutral' };
  }
}

/**
 * The feature test cases that cover a threat, each with its latest imported
 * result and a link to it (ADR 0016, 06-ui.md Threats).
 */
@Component({
  selector: 'tm-covering-tests',
  imports: [RouterLink, IxPill],
  template: `
    <section class="cov" aria-labelledby="cov-title">
      <h3 id="cov-title" class="typography-h6">Covered by test cases</h3>
      @if (error(); as err) {
        <p class="typography-body-sm">{{ err }}</p>
      } @else if (!testCaseIds().length) {
        <p class="typography-body-sm cov__none">
          No feature test case covers this threat yet. Link one from its editor, or convert a security test.
        </p>
      } @else {
        <ul class="cov__list">
          @for (tc of cases(); track tc.id) {
            <li class="cov__item">
              <a class="typography-body" [routerLink]="['/test-cases']" [queryParams]="{ open: tc.id }">{{ tc.name }}</a>
              @let badge = badgeOf(tc);
              <ix-pill [variant]="badge.variant" [outline]="badge.variant === 'neutral'">{{ badge.text }}</ix-pill>
            </li>
          }
        </ul>
      }
    </section>
  `,
  styles: `
    .cov { display: flex; flex-direction: column; gap: var(--si-sys-sizing-spacing-y-20); }
    .cov__none { color: var(--si-sys-color-text-secondary); }
    .cov__list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column;
                 gap: var(--si-sys-sizing-spacing-y-20); }
    .cov__item { display: flex; align-items: center; justify-content: space-between;
                 gap: var(--si-sys-sizing-spacing-x-40); }
    a { color: var(--si-sys-color-text-accent); }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CoveringTestsComponent {
  private readonly api = inject(OrgApi);

  readonly testCaseIds = input<string[]>([]);
  readonly cases = signal<TestCase[]>([]);
  readonly error = signal<string | null>(null);
  readonly badgeOf = resultBadge;
  private requests = 0;

  constructor() {
    effect(() => {
      const ids = this.testCaseIds();
      untracked(() => void this.load(ids));
    });
  }

  private async load(ids: string[]): Promise<void> {
    const request = ++this.requests;
    if (!ids.length) {
      this.cases.set([]);
      return;
    }
    try {
      const cases = await Promise.all(ids.map(id => this.api.getTestCase(id)));
      if (request !== this.requests) return;
      this.cases.set(cases);
      this.error.set(null);
    } catch {
      if (request !== this.requests) return;
      this.error.set('The covering test cases could not be loaded.');
    }
  }
}
