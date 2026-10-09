import {
  ChangeDetectionStrategy, Component, computed, input, output, signal,
} from '@angular/core';
import { AutofocusDirective } from '../../../../shared/components/autofocus';
import {
  StateTest, TestCategory, TestPolarity,
  TEST_CATEGORIES, TEST_POLARITIES, testToGherkin, safeExternalUrl,
} from '../../state/model-editor.store';

/** The editable fields of a test case (id and seq are not user-editable). */
export type TestDraft = Omit<StateTest, 'id' | 'seq'>;

/** Overlay editor for a single Gherkin test case attached to a state. */
@Component({
  selector: 'tm-test-case-dialog',
  imports: [AutofocusDirective],
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

  readonly save   = output<TestDraft>();
  readonly cancelled = output<void>();

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
