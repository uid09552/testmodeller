import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, output, signal,
  untracked,
} from '@angular/core';
import { IxSelect, IxSelectItem } from '@siemens/ix-angular/standalone';
import { AutofocusDirective } from '../../../../shared/components/autofocus';
import {
  EDITOR_PROJECT_ID, ProjectThreats, ThreatOption,
} from '../../../threat-model/state/project-threats';
import {
  StateTest, TestCategory, TestPolarity,
  TEST_CATEGORIES, TEST_POLARITIES, testToGherkin, safeExternalUrl,
} from '../../state/model-editor.store';

/** The editable fields of a test case (id and seq are not user-editable). */
export type TestDraft = Omit<StateTest, 'id' | 'seq'>;

/** Overlay editor for a single Gherkin test case attached to a state. */
@Component({
  selector: 'tm-test-case-dialog',
  imports: [AutofocusDirective, IxSelect, IxSelectItem],
  templateUrl: './test-case-dialog.html',
  styleUrl: './test-case-dialog.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TestCaseDialogComponent {
  /** The test being edited; the dialog works on a local draft until saved. */
  readonly test      = input.required<StateTest>();
  readonly stateName = input<string>('');
  /** Displayed id, e.g. "LoginFlow_3". */
  readonly ref = input<string>('');

  /**
   * Project whose threats the test case may cover (ADR 0016). Falls back to
   * the model editor's project; without one, the Threats field is hidden.
   */
  readonly projectId = input<string | null>(null);

  readonly save   = output<TestDraft>();
  readonly cancelled = output<void>();

  private readonly editorProject = inject(EDITOR_PROJECT_ID, { optional: true });
  private readonly projectThreats = inject(ProjectThreats);
  /** Threats of the project; empty until loaded, or when it has no threat model. */
  readonly threatOptions = signal<ThreatOption[]>([]);
  readonly threatsError = signal<string | null>(null);

  constructor() {
    effect(() => {
      const project = this.projectId() ?? this.editorProject?.() ?? null;
      untracked(() => void this.loadThreats(project));
    });
  }

  private async loadThreats(project: string | null): Promise<void> {
    if (!project) {
      this.threatOptions.set([]);
      return;
    }
    try {
      this.threatOptions.set(await this.projectThreats.options(project));
      this.threatsError.set(null);
    } catch {
      this.threatsError.set('The threats of this project could not be loaded.');
    }
  }

  readonly categories = TEST_CATEGORIES;
  readonly polarities = TEST_POLARITIES;

  // ── Local draft ────────────────────────────────────────────────────────────
  private readonly draft = signal<TestDraft | null>(null);

  private current(): TestDraft {
    const d = this.draft();
    if (d) return d;
    const t = this.test();
    return {
      name: t.name, category: t.category, polarity: t.polarity,
      given: t.given, when: t.when, then: t.then,
      implementationUrl: t.implementationUrl ?? '',
      backlogUrl: t.backlogUrl ?? '',
      ...(t.threatIds ? { threatIds: t.threatIds } : {}),
    };
  }

  readonly name     = computed(() => this.current().name);
  readonly category = computed(() => this.current().category);
  readonly polarity = computed(() => this.current().polarity);
  readonly given    = computed(() => this.current().given);
  readonly when     = computed(() => this.current().when);
  readonly then     = computed(() => this.current().then);
  readonly implementationUrl = computed(() => this.current().implementationUrl ?? '');
  readonly backlogUrl        = computed(() => this.current().backlogUrl ?? '');
  readonly threatIds         = computed(() => this.current().threatIds ?? []);

  /** Null when the field is non-empty but not a usable http(s) link. */
  readonly implementationHref = computed(() => safeExternalUrl(this.implementationUrl()));
  readonly backlogHref        = computed(() => safeExternalUrl(this.backlogUrl()));

  readonly implementationInvalid = computed(() =>
    this.implementationUrl().trim().length > 0 && !this.implementationHref());
  readonly backlogInvalid = computed(() =>
    this.backlogUrl().trim().length > 0 && !this.backlogHref());

  private patch(changes: Partial<TestDraft>): void {
    this.draft.set({ ...this.current(), ...changes });
  }

  setName(v: string):            void { this.patch({ name: v }); }
  setCategory(v: TestCategory):  void { this.patch({ category: v }); }
  setPolarity(v: TestPolarity):  void { this.patch({ polarity: v }); }
  setGiven(v: string):           void { this.patch({ given: v }); }
  setWhen(v: string):            void { this.patch({ when: v }); }
  setThen(v: string):            void { this.patch({ then: v }); }
  setImplementationUrl(v: string): void { this.patch({ implementationUrl: v }); }
  setBacklogUrl(v: string):        void { this.patch({ backlogUrl: v }); }
  /** From `ix-select`'s `valueChange` in multiple mode. */
  setThreats(e: Event): void {
    const v = (e as CustomEvent<string | string[]>).detail;
    this.patch({ threatIds: Array.isArray(v) ? v : v ? [v] : [] });
  }

  /** Live Gherkin preview of the draft. */
  readonly preview = computed(() =>
    testToGherkin({ id: 'preview', seq: this.test().seq, ...this.current() }, this.ref()),
  );

  readonly valid = computed(() =>
    this.current().name.trim().length > 0
    && !this.implementationInvalid()
    && !this.backlogInvalid());

  onSave(): void {
    if (!this.valid()) return;
    this.save.emit(this.current());
  }

  categoryLabel(c: TestCategory): string {
    return { unit: 'Unit', integration: 'Integration', feature: 'Feature' }[c];
  }
}
