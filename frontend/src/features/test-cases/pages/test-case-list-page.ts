import {
  ChangeDetectionStrategy, Component, computed, effect, HostListener, inject, signal, untracked,
} from '@angular/core';
import { OrgApi } from '../../../core/api/org-api';
import { readJson, writeJson } from '../../../core/persistence/local-store';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { fromRemote, PersistedModel, testToInput } from '../../models/state/model-mapping';
import {
  StateTest, TestCategory, gherkinClause, testRefPrefix, safeExternalUrl,
} from '../../models/state/model-editor.store';
import {
  TestCaseDialogComponent, TestDraft,
} from '../../models/components/test-case-dialog/test-case-dialog';
import {
  CATEGORY_ORDER, EMPTY_FILTERS, GroupKey, LinkFilter, SortKey, SortState, TestCaseFilters,
  TestRow, activeFilterCount, facetOptions, filterRows, groupRows, sortRows, toggleSort,
} from '../state/test-case-view';

/** Table layout the user chose; remembered across visits (per browser). */
interface ViewPrefs {
  filters: TestCaseFilters;
  sort: SortState;
  groupBy: GroupKey;
}

const VIEW_KEY = 'test-cases:view';
const DEFAULT_SORT: SortState = { key: 'ref', dir: 'asc' };

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

  // ── Filters, sorting, grouping ────────────────────────────────────────────
  private readonly saved = readJson<Partial<ViewPrefs>>(VIEW_KEY);

  readonly filters = signal<TestCaseFilters>({ ...EMPTY_FILTERS, ...this.saved?.filters });
  readonly sort    = signal<SortState>(this.saved?.sort ?? DEFAULT_SORT);
  readonly groupBy = signal<GroupKey>(this.saved?.groupBy ?? 'none');
  /** Keys of collapsed groups; reset when the grouping changes. */
  readonly collapsed = signal<ReadonlySet<string>>(new Set());
  readonly projectMenuOpen = signal(false);

  readonly categoryFilters: TestCategory[] = CATEGORY_ORDER;
  readonly polarityFilters: TestCaseFilters['polarity'][] = ['all', 'positive', 'negative'];
  readonly linkFilters: { value: LinkFilter; label: string }[] = [
    { value: 'all',           label: 'Any links' },
    { value: 'implemented',   label: 'Has implementation' },
    { value: 'unimplemented', label: 'No implementation' },
    { value: 'backlog',       label: 'Has backlog item' },
    { value: 'none',          label: 'No links' },
  ];
  readonly sortColumns: { key: SortKey; label: string; width: string | null }[] = [
    { key: 'ref',       label: 'ID',        width: '118px' },
    { key: 'component', label: 'Component', width: '130px' },
    { key: 'feature',   label: 'Scenario',  width: '130px' },
    { key: 'model',     label: 'Model',     width: '130px' },
    { key: 'name',      label: 'Test Case', width: null },
    { key: 'category',  label: 'Category',  width: '100px' },
    { key: 'polarity',  label: 'Type',      width: '78px' },
  ];
  readonly groupOptions: { value: GroupKey; label: string }[] = [
    { value: 'none',      label: 'No grouping' },
    { value: 'project',   label: 'Project' },
    { value: 'component', label: 'Component' },
    { value: 'feature',   label: 'Scenario' },
    { value: 'model',     label: 'Model' },
    { value: 'state',     label: 'State' },
    { value: 'category',  label: 'Category' },
    { value: 'polarity',  label: 'Type' },
  ];

  constructor() {
    effect(() => {
      const ids = this.explorer.modelPaths().map(p => p.model.id);
      untracked(() => void this.load(ids));
    });
    effect(() => {
      writeJson(VIEW_KEY, {
        filters: this.filters(), sort: this.sort(), groupBy: this.groupBy(),
      } satisfies ViewPrefs);
    });
  }

  readonly projects = computed(() =>
    this.explorer.projects().map(p => ({ id: p.id, name: p.name })));

  readonly selectedProjectName = computed(() => {
    const id = this.filters().projectId;
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
          componentId: path.componentId,
          componentName: path.componentName,
          featureId: path.featureId,
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

  readonly projectCounts = computed(() => {
    const counts = new Map<string, number>();
    for (const r of this.allRows()) counts.set(r.projectId, (counts.get(r.projectId) ?? 0) + 1);
    return counts;
  });

  readonly facets = computed(() => facetOptions(this.allRows(), this.filters()));

  /** Filtered and sorted — the set that is shown, counted and exported. */
  readonly rows = computed(() => sortRows(filterRows(this.allRows(), this.filters()), this.sort()));

  readonly groups = computed(() => groupRows(this.rows(), this.groupBy()));

  readonly activeFilters = computed(() => activeFilterCount(this.filters()));

  patchFilters(changes: Partial<TestCaseFilters>): void {
    this.filters.update(f => ({ ...f, ...changes }));
  }

  /** Picking a level clears the levels below it, which may no longer exist under it. */
  setProject(id: string): void {
    this.patchFilters({ projectId: id, componentId: 'all', featureId: 'all', modelId: 'all' });
    this.projectMenuOpen.set(false);
  }
  setComponent(id: string): void { this.patchFilters({ componentId: id, featureId: 'all', modelId: 'all' }); }
  setFeature(id: string):   void { this.patchFilters({ featureId: id, modelId: 'all' }); }
  setModel(id: string):     void { this.patchFilters({ modelId: id }); }

  toggleCategory(c: TestCategory): void {
    const cur = this.filters().categories;
    this.patchFilters({ categories: cur.includes(c) ? cur.filter(x => x !== c) : [...cur, c] });
  }

  clearFilters(): void { this.filters.set(EMPTY_FILTERS); }

  sortBy(key: SortKey): void { this.sort.update(s => toggleSort(s, key)); }

  ariaSort(key: SortKey): 'ascending' | 'descending' | 'none' {
    const s = this.sort();
    if (s.key !== key) return 'none';
    return s.dir === 'asc' ? 'ascending' : 'descending';
  }

  setGroupBy(g: GroupKey): void {
    this.groupBy.set(g);
    this.collapsed.set(new Set());
  }

  isCollapsed(key: string): boolean { return this.collapsed().has(key); }

  toggleGroup(key: string): void {
    this.collapsed.update(set => {
      const next = new Set(set);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  }

  collapseAll(): void { this.collapsed.set(new Set(this.groups().map(g => g.key))); }
  expandAll():   void { this.collapsed.set(new Set()); }

  // ── Summary ────────────────────────────────────────────────────────────────
  readonly positiveCount = computed(() => this.rows().filter(r => r.test.polarity === 'positive').length);
  readonly negativeCount = computed(() => this.rows().filter(r => r.test.polarity === 'negative').length);
  readonly modelCount    = computed(() => new Set(this.rows().map(r => r.modelId)).size);
  readonly implementedCount = computed(() =>
    this.rows().filter(r => !!r.test.implementationUrl?.trim()).length);

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
