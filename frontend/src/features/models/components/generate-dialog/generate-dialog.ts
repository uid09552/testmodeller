import {
  ChangeDetectionStrategy, Component, computed, inject, OnInit, signal,
} from '@angular/core';
import {
  IxActiveModal, IxButton, IxInput, IxMessageBar, IxModalContent, IxModalFooter, IxModalHeader,
  IxNumberInput, IxSelect, IxSelectItem, IxSpinner,
} from '@siemens/ix-angular/standalone';
import { ApiError } from '../../../../core/api/api-error';
import { TestCase } from '../../../../core/api/api.types';
import { CoverageCriterion, GenerationResult, OrgApi } from '../../../../core/api/org-api';
import { diffGeneration } from './generation-diff';

/** What the dialog is opened with. */
export interface GenerateDialogData {
  modelId: string;
  modelName: string;
}

/** How the dialog closed: `saved` lists the stored test cases. */
export interface GenerateDialogResult {
  saved: TestCase[];
}

export const CRITERIA: { value: CoverageCriterion; label: string; hint: string }[] = [
  { value: 'transition', label: 'All transitions', hint: 'Every transition at least once — the usual choice.' },
  { value: 'state', label: 'All states', hint: 'Every state at least once — the fewest test cases.' },
  { value: 'transition-pair', label: 'Transition pairs', hint: 'Every pair of consecutive transitions — catches ordering faults.' },
  { value: 'bounded-paths', label: 'All paths up to a length', hint: 'Every path up to the maximum length — thorough, can be many.' },
  { value: 'prime-paths', label: 'Prime paths', hint: 'Every longest loop-free path — covers loops without listing every path.' },
  { value: 'reach-element', label: 'Path to one element', hint: 'The shortest path to one state or transition — a single scenario.' },
  { value: 'random', label: 'Random walks', hint: 'Seeded random walks; transition weights set how likely each step is.' },
];

/** A state or transition the generator can aim at. */
export interface Element { id: string; label: string }

/**
 * "Generate test cases" (06-ui.md, FR-010): choose a criterion, preview what
 * generation produces and how it differs from the model's current generated
 * test cases, then save. Nothing is stored until Save.
 */
@Component({
  selector: 'tm-generate-dialog',
  imports: [
    IxButton, IxInput, IxMessageBar, IxModalContent, IxModalFooter, IxModalHeader, IxNumberInput,
    IxSelect, IxSelectItem, IxSpinner,
  ],
  templateUrl: './generate-dialog.html',
  styleUrl: './generate-dialog.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GenerateDialogComponent implements OnInit {
  private readonly modal = inject<IxActiveModal<GenerateDialogData, GenerateDialogResult | null>>(IxActiveModal);
  private readonly api = inject(OrgApi);

  readonly data = this.modal.data!;
  readonly criteria = CRITERIA;
  readonly criterion = signal<CoverageCriterion>('transition');
  readonly maxPathLength = signal(8);
  /** Empty: let the server choose, and show the seed it used. */
  readonly seed = signal('');
  /** `reach-element`: the chosen state or transition. */
  readonly targetId = signal<string | null>(null);
  readonly elements = signal<Element[]>([]);
  /** `random`: stop conditions. */
  readonly stopAfterTests = signal(10);
  readonly stopAtCoverage = signal<number | null>(null);

  readonly existing = signal<TestCase[]>([]);
  readonly result = signal<GenerationResult | null>(null);
  readonly busy = signal<'preview' | 'save' | null>(null);
  readonly error = signal<string | null>(null);

  readonly hint = computed(() => CRITERIA.find(c => c.value === this.criterion())!.hint);
  readonly seedInvalid = computed(() => this.seed().trim() !== '' && !/^-?\d+$/.test(this.seed().trim()));
  readonly lengthInvalid = computed(() =>
    this.criterion() === 'bounded-paths' && !(this.maxPathLength() >= 1 && this.maxPathLength() <= 50));
  readonly targetMissing = computed(() => this.criterion() === 'reach-element' && !this.targetId());
  readonly stopInvalid = computed(() => this.criterion() === 'random'
    && (!(this.stopAfterTests() >= 1 && this.stopAfterTests() <= 1000)
      || (this.stopAtCoverage() !== null && !(this.stopAtCoverage()! >= 1 && this.stopAtCoverage()! <= 100))));
  readonly canRun = computed(() => !this.seedInvalid() && !this.lengthInvalid()
    && !this.targetMissing() && !this.stopInvalid() && !this.busy());
  readonly diff = computed(() => {
    const r = this.result();
    return r ? diffGeneration(this.existing(), r.testCases) : null;
  });

  async ngOnInit(): Promise<void> {
    try {
      const cases = await this.api.modelTestCases(this.data.modelId);
      this.existing.set(cases.filter(tc => tc.origin === 'generated' && tc.generatedFromModelId === this.data.modelId));
    } catch {
      // The diff is a convenience; generation still works without it.
    }
    try {
      const model = await this.api.getModel(this.data.modelId);
      const names = new Map(model.states.map(s => [s.id, s.name] as const));
      this.elements.set([
        ...model.states.filter(s => s.id).map(s => ({ id: s.id!, label: `State: ${s.name}` })),
        ...model.transitions.filter(t => t.id).map(t => ({
          id: t.id!,
          label: `Transition: ${names.get(t.from) ?? '?'} → ${names.get(t.to) ?? '?'} on "${t.event}"`,
        })),
      ]);
    } catch {
      // Only the element picker of reach-element needs it; the dialog reports a missing target.
    }
  }

  setTarget(e: Event): void {
    const v = (e as CustomEvent<string | string[]>).detail;
    const id = Array.isArray(v) ? v[0] : v;
    this.targetId.set(this.elements().some(x => x.id === id) ? id : null);
    this.result.set(null);
  }

  setStopAfter(e: Event): void {
    this.stopAfterTests.set(Number((e as CustomEvent<number>).detail));
    this.result.set(null);
  }

  setStopCoverage(e: Event): void {
    const v = (e as CustomEvent<string | number>).detail;
    this.stopAtCoverage.set(v === '' || v === null || v === undefined ? null : Number(v));
    this.result.set(null);
  }

  setCriterion(e: Event): void {
    const v = (e as CustomEvent<string | string[]>).detail;
    const c = (Array.isArray(v) ? v[0] : v) as CoverageCriterion;
    if (CRITERIA.some(x => x.value === c)) {
      this.criterion.set(c);
      this.result.set(null);
    }
  }

  setMaxPathLength(e: Event): void {
    this.maxPathLength.set(Number((e as CustomEvent<number>).detail));
    this.result.set(null);
  }

  setSeed(e: Event): void {
    this.seed.set(String((e as CustomEvent<string>).detail ?? ''));
    this.result.set(null);
  }

  /**
   * The seed of the last preview when none was typed. Chosen here, not by the
   * server: a server-chosen seed is a 64-bit number that JavaScript cannot
   * hold exactly, so Save could not repeat the preview with it.
   */
  private autoSeed: number | null = null;

  /** The request; Save repeats the preview's seed, so it stores exactly what was shown. */
  private request(save: boolean) {
    const typed = this.seed().trim();
    if (typed === '' && (!save || this.autoSeed === null)) {
      this.autoSeed = Math.floor(Math.random() * 2 ** 31);
    }
    const seed = typed !== '' ? Number(typed) : this.autoSeed!;
    return {
      criterion: this.criterion(),
      ...(seed !== undefined ? { seed } : {}),
      ...(this.criterion() === 'bounded-paths' ? { maxPathLength: this.maxPathLength() } : {}),
      ...(this.criterion() === 'reach-element' ? { targetId: this.targetId()! } : {}),
      ...(this.criterion() === 'random' ? {
        stopAfterTests: this.stopAfterTests(),
        ...(this.stopAtCoverage() !== null ? { stopAtCoverage: this.stopAtCoverage()! } : {}),
      } : {}),
      save,
    };
  }

  async preview(): Promise<void> {
    await this.run('preview', async () => this.result.set(await this.api.generate(this.data.modelId, this.request(false))));
  }

  async save(): Promise<void> {
    await this.run('save', async () => {
      const stored = await this.api.generate(this.data.modelId, this.request(true));
      this.modal.close({ saved: stored.testCases });
    });
  }

  cancel(): void {
    this.modal.dismiss(null);
  }

  private async run(kind: 'preview' | 'save', work: () => Promise<void>): Promise<void> {
    if (!this.canRun()) return;
    this.busy.set(kind);
    this.error.set(null);
    try {
      await work();
    } catch (e) {
      this.error.set(e instanceof ApiError ? e.message : 'Generation failed. Check the model for validation errors.');
    } finally {
      this.busy.set(null);
    }
  }

  pct(c: { covered: number; total: number }): number {
    return c.total ? Math.round((c.covered / c.total) * 100) : 100;
  }
}
