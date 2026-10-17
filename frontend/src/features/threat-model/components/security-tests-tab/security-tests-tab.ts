import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked, viewChild,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  SecurityTest, SecurityTestInput, SecurityTestOutcome, SecurityTestStatus, SecurityTestType,
} from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { testInput } from '../../state/inputs';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import {
  download, label, securityTestsCsv, securityTestsGherkin, aiMark } from '../../state/threat-model-view';
import { AiBadgeComponent } from '../ai-badge/ai-badge';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

/** Latest result as a symbol and a word. */
export function resultText(t: SecurityTest): string {
  switch (t.lastResult?.outcome) {
    case 'passed': return '✓ Passed';
    case 'failed': return '✗ Failed';
    case 'not-run': return '– Not run';
    default: return '—';
  }
}

type TestDraft = SecurityTestInput & { preconditionsText: string };

/** Security tests of the threat model, separate from feature test cases (FR-073..FR-075). */
@Component({
  selector: 'tm-security-tests-tab',
  imports: [AiBadgeComponent, FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './security-tests-tab.html',
  styleUrl: './security-tests-tab.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SecurityTestsTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly selectId = input<string | null>(null);
  readonly types: SecurityTestType[] = [
    'negative-authorization', 'authentication', 'input-validation', 'transport-security',
    'configuration-review', 'resilience', 'logging-audit', 'other',
  ];
  readonly statuses: SecurityTestStatus[] = ['draft', 'ready', 'obsolete'];
  readonly outcomes: SecurityTestOutcome[] = ['passed', 'failed', 'not-run'];
  readonly label = label;
  readonly resultText = resultText;

  readonly status = signal('');
  readonly type = signal('');
  readonly result = signal('');
  readonly checked = signal<string[]>([]);
  readonly message = signal<string | null>(null);
  private readonly table = viewChild<EntityTableComponent<SecurityTest>>(EntityTableComponent);

  /** The test whose result is being recorded. */
  readonly recording = signal<SecurityTest | null>(null);
  readonly outcome = signal<SecurityTestOutcome>('passed');
  readonly note = signal('');
  readonly recordError = signal<string | null>(null);

  readonly threatOptions = computed(() =>
    this.store.threats().filter(t => t.status !== 'dismissed' && t.status !== 'candidate')
      .map(t => ({ id: t.id, label: `${t.displayId} ${t.title}` })));
  readonly controlOptions = computed(() =>
    this.store.controls().map(c => ({ id: c.id, label: `${c.displayId} ${c.name}` })));

  readonly rows = computed(() => this.store.tests().filter(t =>
    (!this.status() || t.status === this.status())
    && (!this.type() || t.type === this.type())
    && (!this.result() || (t.lastResult?.outcome ?? 'none') === this.result())));

  private ids(ids: string[], items: { id: string; displayId: string }[]): string {
    return ids.map(id => items.find(x => x.id === id)?.displayId ?? '?').join(', ') || '—';
  }

  readonly columns = computed<Column<SecurityTest>[]>(() => [
    { key: 'title', label: 'Test', text: t => `${t.displayId} ${t.title}${aiMark(t)}${t.stale ? ` ⚠ Stale: ${t.stale}` : ''}`,
      sort: t => Number(t.displayId.slice(4)) },
    { key: 'type', label: 'Type', text: t => label(t.type) },
    { key: 'threats', label: 'Threats', text: t => this.ids(t.threatIds, this.store.threats()) },
    { key: 'controls', label: 'Controls', text: t => this.ids(t.controlIds, this.store.controls()) },
    { key: 'status', label: 'Status', text: t => label(t.status) + (t.origin === 'generated' ? ' (generated)' : '') },
    { key: 'result', label: 'Latest result', text: t => resultText(t) },
  ]);

  readonly editor = new EditorState<SecurityTest, TestDraft>(
    (d, item) => {
      const { preconditionsText, ...input } = d;
      return this.store.save('security-tests', {
        ...input,
        objective: orNull(input.objective),
        preconditions: preconditionsText.split('\n').map(s => s.trim()).filter(Boolean),
        steps: input.steps.filter(s => s.action.trim()),
      }, item ?? undefined);
    },
    t => this.store.remove('security-tests', t.id),
  );

  constructor() {
    effect(() => {
      const t = this.store.tests().find(x => x.id === this.selectId());
      if (t) untracked(() => this.edit(t));
    });
  }

  add(): void {
    this.editor.open(null, {
      title: '', objective: '', preconditions: [], preconditionsText: '',
      steps: [{ action: '', expected: '' }], type: 'negative-authorization', status: 'draft',
      threatIds: [], controlIds: [],
    });
  }

  edit(t: SecurityTest): void {
    this.editor.open(t, { ...testInput(t), preconditionsText: t.preconditions.join('\n') });
  }

  addStep(d: TestDraft): void {
    d.steps = [...d.steps, { action: '', expected: '' }];
  }

  removeStep(d: TestDraft, i: number): void {
    d.steps = d.steps.filter((_, j) => j !== i);
  }

  async generate(): Promise<void> {
    this.message.set(null);
    const selected = this.store.threats().filter(t => this.selectedThreats().includes(t.id)).map(t => t.id);
    try {
      const r = await this.store.generateTests(selected.length ? selected : undefined);
      this.message.set(`${r.created.length} draft test(s) generated` + (r.skipped ? `, ${r.skipped} skipped (already exist)` : '') + '. Review them and mark them ready.');
      if (r.created.length) this.status.set('draft');
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  /** Threats chosen for generation; empty means all open and accepted ones. */
  readonly selectedThreats = signal<string[]>([]);

  toggleThreat(id: string): void {
    this.selectedThreats.update(s => (s.includes(id) ? s.filter(x => x !== id) : [...s, id]));
  }

  async markReady(): Promise<void> {
    const ids = this.checked();
    if (!ids.length) return;
    try {
      await this.store.bulkTestStatus(ids, 'ready');
      this.table()?.clearChecked();
      this.message.set(`${ids.length} test(s) marked ready.`);
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  startRecording(t: SecurityTest): void {
    this.recordError.set(null);
    this.outcome.set('passed');
    this.note.set('');
    this.recording.set(t);
  }

  async record(): Promise<void> {
    const t = this.recording();
    if (!t) return;
    try {
      await this.store.recordResult(t.id, this.outcome(), this.note().trim() || undefined);
      this.recording.set(null);
      this.editor.close();
    } catch (e) {
      this.recordError.set(errorMessage(e));
    }
  }

  /** The tests the table currently shows (filters and text filter applied). */
  private shown(): SecurityTest[] {
    return this.table()?.visible() ?? this.rows();
  }

  exportCsv(): void {
    download('security-tests.csv', securityTestsCsv(this.shown(), this.store.threats()), 'text/csv');
  }

  exportGherkin(): void {
    const name = this.store.model()?.name ?? 'Security tests';
    download('security-tests.feature', securityTestsGherkin(this.shown(), this.store.threats(), name), 'text/plain');
  }
}
