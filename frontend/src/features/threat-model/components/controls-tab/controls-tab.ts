import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  Control, ControlCategory, ControlInput, ControlStatus, ControlType,
} from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { controlInput } from '../../state/inputs';
import { ThreatModelStore } from '../../state/threat-model.store';
import { label } from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

type Gap = '' | 'no-threats' | 'no-tests';

/** Security controls and what they cover (FR-072). */
@Component({
  selector: 'tm-controls-tab',
  imports: [FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './controls-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ControlsTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly nav = inject(WorkspaceNav);
  readonly selectId = input<string | null>(null);
  readonly types: ControlType[] = ['preventive', 'detective', 'corrective', 'compensating'];
  readonly categories: ControlCategory[] = [
    'identity-access', 'data-protection', 'application-network', 'detection-response', 'secure-development',
  ];
  readonly statuses: ControlStatus[] = ['planned', 'implemented', 'verified'];
  readonly label = label;
  readonly gap = signal<Gap>('');
  readonly status = signal('');

  readonly componentOptions = computed(() => this.store.components().map(c => ({ id: c.id, label: c.name })));
  readonly threatOptions = computed(() =>
    this.store.threats().filter(t => t.status !== 'dismissed')
      .map(t => ({ id: t.id, label: `${t.displayId} ${t.title}` })));

  readonly rows = computed(() => this.store.controls().filter(c =>
    (!this.status() || c.status === this.status())
    && (this.gap() !== 'no-threats' || c.threatIds.length === 0)
    && (this.gap() !== 'no-tests' || c.securityTestIds.length === 0)));

  private threatIds(c: Control): string {
    return c.threatIds.map(id => this.store.threats().find(t => t.id === id)?.displayId ?? '?').join(', ') || '—';
  }

  readonly columns = computed<Column<Control>[]>(() => [
    { key: 'name', label: 'Control', text: c => `${c.displayId} ${c.name}`, sort: c => Number(c.displayId.slice(4)) },
    { key: 'type', label: 'Type', text: c => label(c.type) },
    { key: 'category', label: 'Category', text: c => label(c.category) },
    { key: 'status', label: 'Status', text: c => label(c.status) + (c.verifiedAt ? ` (${c.verifiedAt.slice(0, 10)})` : '') },
    { key: 'threats', label: 'Threats', text: c => this.threatIds(c) },
    { key: 'tests', label: 'Security tests', text: c => `${c.securityTestIds.length}`, sort: c => c.securityTestIds.length },
  ]);

  readonly editor = new EditorState<Control, ControlInput>(
    (d, item) => this.store.save('controls', {
      ...d, description: orNull(d.description), owner: orNull(d.owner),
      acceptanceCriteria: orNull(d.acceptanceCriteria),
    }, item ?? undefined),
    c => this.store.remove('controls', c.id),
  );

  constructor() {
    effect(() => {
      const c = this.store.controls().find(x => x.id === this.selectId());
      if (c) untracked(() => this.edit(c));
    });
  }

  add(): void {
    this.editor.open(null, {
      name: '', description: '', type: 'preventive', category: 'identity-access', status: 'planned',
      owner: '', acceptanceCriteria: '', componentIds: [], threatIds: [],
    });
  }

  edit(c: Control): void {
    this.editor.open(c, controlInput(c));
  }

  testsOf(c: Control) {
    return this.store.tests().filter(t => c.securityTestIds.includes(t.id));
  }
}
