import {
  ChangeDetectionStrategy, Component, computed, HostListener, inject, signal,
} from '@angular/core';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { ModelRepository } from '../../models/state/model-repository';
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
  private readonly repo     = inject(ModelRepository);

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
    const stored = this.repo.all();
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
    const model = this.repo.get(ids.modelId);
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

  /** Write the edited test straight back through the repository. */
  saveTest(changes: TestDraft): void {
    const ids = this.editingIds();
    if (!ids) return;
    const model = this.repo.get(ids.modelId);
    if (!model) { this.closeEditor(); return; }

    this.repo.save({
      ...model,
      nodes: model.nodes.map(n =>
        n.id !== ids.nodeId ? n : {
          ...n,
          tests: (n.tests ?? []).map(t => t.id === ids.testId ? { ...t, ...changes } : t),
        },
      ),
    });
    this.closeEditor();
  }

  removeRow(row: TestRow): void {
    const model = this.repo.get(row.modelId);
    if (!model) return;
    this.repo.save({
      ...model,
      nodes: model.nodes.map(n =>
        n.id !== row.nodeId ? n : {
          ...n,
          tests: (n.tests ?? []).filter(t => t.id !== row.test.id),
        },
      ),
    });
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
