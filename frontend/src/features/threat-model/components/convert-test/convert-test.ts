import {
  ChangeDetectionStrategy, Component, computed, inject, input, output, signal,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import {
  IxButton, IxSelect, IxSelectItem, MessageService, ToastService,
} from '@siemens/ix-angular/standalone';
import { ApiError } from '../../../../core/api/api-error';
import { TestCase } from '../../../../core/api/api.types';
import { SecurityTest, ThreatModelApi } from '../../../../core/api/threat-model-api';
import { ExplorerStore } from '../../../explorer/state/explorer.store';

/** A model the security test can become a test case of. */
export interface ModelTarget {
  modelId: string;
  featureId: string;
  /** "Component › Feature › Model". */
  label: string;
}

/**
 * "Convert to feature test case" in the security test editor (ADR 0016,
 * 06-ui.md Security Tests). Choosing a model and confirming copies the test
 * into a test case of that model's feature, assigned to its initial state.
 */
@Component({
  selector: 'tm-convert-test',
  imports: [RouterLink, IxButton, IxSelect, IxSelectItem],
  template: `
    <section class="conv" aria-labelledby="conv-title">
      <h3 id="conv-title" class="typography-h6">Feature test case</h3>
      @if (test().convertedToTestCaseId; as tcId) {
        <p class="typography-body">
          Converted —
          <a [routerLink]="['/test-cases']" [queryParams]="{ open: tcId }">open the feature test case</a>.
          This security test is now obsolete.
        </p>
      } @else if (!targets().length) {
        <p class="typography-body-sm conv__hint">
          Add a model to a feature of this project to turn this check into a model-based test case.
        </p>
      } @else {
        <p class="typography-body-sm conv__hint">
          Copy this test into a feature test case that runs with the model-based tests and covers the
          same threats. This security test then becomes obsolete.
        </p>
        <div class="conv__row">
          <ix-select class="conv__select" label="Model" [value]="modelId() ?? ''"
                     (valueChange)="choose($event)">
            @for (t of targets(); track t.modelId) {
              <ix-select-item [value]="t.modelId" [label]="t.label" />
            }
          </ix-select>
          <ix-button variant="secondary" [loading]="busy()" [disabled]="!modelId()"
                     (click)="convert()">Convert to feature test case</ix-button>
        </div>
      }
    </section>
  `,
  styles: `
    .conv { display: flex; flex-direction: column; gap: var(--si-sys-sizing-spacing-y-20); }
    .conv__hint { color: var(--si-sys-color-text-secondary); }
    .conv__row { display: flex; flex-wrap: wrap; align-items: flex-end; gap: var(--si-sys-sizing-spacing-x-40); }
    .conv__select { flex: 1; min-width: var(--si-sys-sizing-size-160); }
    a { color: var(--si-sys-color-text-accent); }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConvertTestComponent {
  private readonly api = inject(ThreatModelApi);
  private readonly explorer = inject(ExplorerStore);
  private readonly messages = inject(MessageService);
  private readonly toasts = inject(ToastService);

  readonly threatModelId = input.required<string>();
  readonly projectId = input.required<string | null>();
  readonly test = input.required<SecurityTest>();
  readonly converted = output<TestCase>();

  readonly modelId = signal<string | null>(null);
  readonly busy = signal(false);

  /** Models of the threat model's project. */
  readonly targets = computed<ModelTarget[]>(() =>
    this.explorer.modelPaths()
      .filter(p => p.projectId === this.projectId())
      .map(p => ({
        modelId: p.model.id,
        featureId: p.featureId,
        label: `${p.componentName} › ${p.featureName} › ${p.model.name}`,
      })));

  choose(e: Event): void {
    const v = (e as CustomEvent<string | string[]>).detail;
    const id = Array.isArray(v) ? v[0] : v;
    this.modelId.set(this.targets().some(t => t.modelId === id) ? id : null);
  }

  async convert(): Promise<void> {
    const target = this.targets().find(t => t.modelId === this.modelId());
    if (!target || !(await this.confirm(target))) return;
    this.busy.set(true);
    try {
      const tc = await this.api.convertTest(
        this.threatModelId(), this.test().id, target.featureId, target.modelId);
      void this.toasts.show({
        type: 'success',
        message: `${this.test().displayId} is now the test case "${tc.name}".`,
      });
      this.converted.emit(tc);
    } catch (e) {
      void this.toasts.show({
        type: 'error',
        message: e instanceof ApiError ? e.message : 'The security test could not be converted.',
      });
    } finally {
      this.busy.set(false);
    }
  }

  private confirm(target: ModelTarget): Promise<boolean> {
    return new Promise(resolve => {
      void this.messages
        .information(
          'Convert to feature test case?',
          `${this.test().displayId} becomes a test case of ${target.label}, assigned to its initial `
            + 'state and covering the same threats. The security test is then obsolete.',
          'Convert', 'Cancel', 'convert', 'cancel',
        )
        .then(event => event.once(result => resolve(result.payload === 'convert')));
    });
  }
}
