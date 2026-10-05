import {
  ChangeDetectionStrategy, Component, computed, effect, HostListener, inject, signal, untracked,
} from '@angular/core';
import { OrgApi } from '../../../core/api/org-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { fromRemote, PersistedModel, testToInput } from '../../models/state/model-mapping';
import {
  StateTest, TestCategory, gherkinClause, testRefPrefix, safeExternalUrl,
} from '../../models/state/model-editor.store';
import {
  TestCaseDialogComponent, TestDraft,
} from '../../models/components/test-case-dialog/test-case-dialog';

/** One row of the cross-project test case table. */
interface TestRow {
  projectId: string;
  projectName: string;
  componentName: string;
  featureName: string;
  modelId: string;
  modelName: string;
  stateLabel: string;
  nodeId: string;
  ref: string;
  test: StateTest;
}

type CategoryFilter = 'all' | TestCategory;
type PolarityFilter = 'all' | 'positive' | 'negative';

@Component({
  selector: 'tm-test-case-list-page',
  imports: [TestCaseDialogComponent],
  templateUrl: './test-case-list-page.html',
  styleUrl: './test-case-list-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TestCaseListPageComponent {
  private readonly explorer = inject(ExplorerStore);
  private readonly api      = inject(OrgApi);

  /** Every model's content, as stored in the database. */
  private readonly stored = signal<Record<string, PersistedModel>>({});
  readonly error = signal<string | null>(null);

  constructor() {
    effect(() => {
      const ids = this.explorer.modelPaths().map(p => p.model.id);
      untracked(() => void this.load(ids));
    });
  }

  private async load(ids: string[]): Promise<void> {
    try {
      const entries = await Promise.all(ids.map(id => this.fetchModel(id)));
      this.stored.set(Object.fromEntries(entries.map(m => [m.id, m])));
      this.error.set(null);
    } catch {
      this.error.set('The test cases could not be loaded.');
    }
  }

  private async fetchModel(id: string): Promise<PersistedModel> {
    const [model, testCases] = await Promise.all([
      this.api.getModel(id), this.api.modelTestCases(id),
    ]);
    return { ...fromRemote(model, testCases, id, '', null), id };
  }

  private async reloadModel(id: string): Promise<void> {
    try {
      const model = await this.fetchModel(id);
      this.stored.update(s => ({ ...s, [id]: model }));
    } catch {
      this.error.set('The test cases could not be loaded.');
    }
  }

  // ── Filters ────────────────────────────────────────────────────────────────
  readonly projectFilter  = signal<string>('all');
  readonly categoryFilter = signal<CategoryFilter>('all');
  readonly polarityFilter = signal<PolarityFilter>('all');
  readonly search         = signal('');
  readonly projectMenuOpen = signal(false);

  readonly categoryFilters: CategoryFilter[] = ['all', 'unit', 'integration', 'feature'];
  readonly polarityFilters: PolarityFilter[] = ['all', 'positive', 'negative'];

  readonly projects = computed(() =>
    this.explorer.projects().map(p => ({ id: p.id, name: p.name })));

  readonly selectedProjectName = computed(() => {
    const id = this.projectFilter();
    if (id === 'all') return 'All projects';
    return this.projects().find(p => p.id === id)?.name ?? 'All projects';
  });

  /**
   * Join the Explorer hierarchy with stored model content so every test case
   * carries its full Component > Feature > Model path.
   */
  readonly allRows = computed<TestRow[]>(() => {
    const stored = this.stored();
    return this.explorer.modelPaths().flatMap(path => {
      const model = stored[path.model.id];
      if (!model) return [];
      const prefix = testRefPrefix(model.name);
      return model.nodes.flatMap(node =>
        (node.tests ?? []).map(test => ({
          projectId: path.projectId,
          projectName: path.projectName,
          componentName: path.componentName,
          featureName: path.featureName,
          modelId: path.model.id,
          modelName: model.name,
          stateLabel: node.label,
          nodeId: node.id,
          ref: `${prefix}_${test.seq}`,
          test,
        })),
      );
    });
  });

  readonly rows = computed(() => {
    const proj = this.projectFilter();
    const cat  = this.categoryFilter();
    const pol  = this.polarityFilter();
    const q    = this.search().toLowerCase().trim();

    return this.allRows().filter(r =>
      (proj === 'all' || r.projectId === proj) &&
      (cat  === 'all' || r.test.category === cat) &&
      (pol  === 'all' || r.test.polarity === pol) &&
      (!q ||
        r.ref.toLowerCase().includes(q) ||
        r.test.name.toLowerCase().includes(q) ||
        r.modelName.toLowerCase().includes(q) ||
        r.featureName.toLowerCase().includes(q) ||
        r.componentName.toLowerCase().includes(q)),
    );
  });

  // ── Summary ────────────────────────────────────────────────────────────────
  readonly positiveCount = computed(() => this.rows().filter(r => r.test.polarity === 'positive').length);
  readonly negativeCount = computed(() => this.rows().filter(r => r.test.polarity === 'negative').length);
  readonly modelCount    = computed(() => new Set(this.rows().map(r => r.modelId)).size);

  setProject(id: string): void {
    this.projectFilter.set(id);
    this.projectMenuOpen.set(false);
  }

  @HostListener('document:click', ['$event'])
  onDocClick(e: MouseEvent): void {
    if (this.projectMenuOpen() && !(e.target as Element).closest('.proj-select')) {
      this.projectMenuOpen.set(false);
    }
  }

  @HostListener('document:keydown.escape')
  onEscape(): void { this.projectMenuOpen.set(false); }

  implHref(t: StateTest):    string | null { return safeExternalUrl(t.implementationUrl); }
  backlogHref(t: StateTest): string | null { return safeExternalUrl(t.backlogUrl); }

  /** One-line Given/When/Then digest. */
  summary(test: StateTest): string {
    const part = (kw: string, body: string) => {
      const first = body.split('\n').map(l => l.trim()).find(Boolean);
      return first ? `${kw} ${first}` : '';
    };
    return [part('Given', test.given), part('When', test.when), part('Then', test.then)]
      .filter(Boolean).join('  ·  ') || 'No steps yet';
  }

  // ── Gherkin overlay (double-click a row) ──────────────────────────────────
  readonly editingIds = signal<{ modelId: string; nodeId: string; testId: string } | null>(null);

  readonly editing = computed(() => {
    const ids = this.editingIds();
    if (!ids) return null;
    const model = this.stored()[ids.modelId];
    const node  = model?.nodes.find(n => n.id === ids.nodeId);
    const test  = node?.tests?.find(t => t.id === ids.testId);
    if (!model || !node || !test) return null;
    return {
      test,
      stateName: `${model.name} › ${node.label}`,
      ref: `${testRefPrefix(model.name)}_${test.seq}`,
    };
  });

  editRow(row: TestRow): void {
    this.editingIds.set({ modelId: row.modelId, nodeId: row.nodeId, testId: row.test.id });
  }

  closeEditor(): void { this.editingIds.set(null); }

  /** Writes the edited test through the API, keeping its other assignments. */
  async saveTest(changes: TestDraft): Promise<void> {
    const ids = this.editingIds();
    if (!ids) return;
    const test = this.editing()?.test;
    this.closeEditor();
    if (!test) return;
    try {
      const current = await this.api.getTestCase(test.id);
      const input = {
        ...testToInput({ ...test, ...changes }, ids.modelId, ids.nodeId),
        assignments: current.assignments.map(({ testCaseId: _id, ...a }) => a),
      };
      await this.api.replaceTestCase(current.id, current.version, input);
      await this.reloadModel(ids.modelId);
    } catch {
      this.error.set('The test case could not be saved.');
    }
  }

  async removeRow(row: TestRow): Promise<void> {
    try {
      await this.api.deleteTestCase(row.test.id);
      await this.reloadModel(row.modelId);
    } catch {
      this.error.set('The test case could not be deleted.');
    }
  }

  // ── Export ─────────────────────────────────────────────────────────────────
  exportGherkin(): void {
    const rows = this.rows();
    if (!rows.length) return;
    const byModel = new Map<string, TestRow[]>();
    for (const r of rows) {
      const list = byModel.get(r.modelName) ?? [];
      list.push(r);
      byModel.set(r.modelName, list);
    }

    const text = [...byModel.entries()].map(([modelName, list]) => {
      const scenarios = list.map(r => [
        `  @${r.test.category} @${r.test.polarity} @id:${r.ref}`,
        `  # ${r.componentName} > ${r.featureName} > ${r.stateLabel}`,
        `  Scenario: ${r.test.name}`,
        ...gherkinClause('Given', r.test.given),
        ...gherkinClause('When',  r.test.when),
        ...gherkinClause('Then',  r.test.then),
      ].join('\n')).join('\n\n');
      return `Feature: ${modelName}\n\n${scenarios}\n`;
    }).join('\n');

    const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = 'test-cases.feature';
    a.click();
    URL.revokeObjectURL(url);
  }
}
