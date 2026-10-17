import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, TemplateRef, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Classification, Flow, FlowInput, FlowRelationship, TriState } from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { ThreatModelStore } from '../../state/threat-model.store';
import { label, targetName, aiMark } from '../../state/threat-model-view';
import { AiBadgeComponent } from '../ai-badge/ai-badge';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

export const RELATIONSHIPS: FlowRelationship[] = [
  'calls', 'sends-data', 'reads-writes', 'authenticates-through', 'trusts', 'depends-on',
  'runs-on', 'publishes-subscribes', 'replicates-backs-up', 'deploys',
];

/** How a crossing reads in a list: "⇢ crosses (0 → 3)". */
export function crossingText(f: Flow): string {
  if (!f.crossing.crosses) return 'No';
  const t = (n: number | null) => (n == null ? 'no zone' : `${n}`);
  return `⇢ Crosses (${t(f.crossing.fromTrust)} → ${t(f.crossing.toTrust)})`;
}

type AuthMode = 'unknown' | 'none' | 'method';
type FlowDraft = FlowInput & { authMode: AuthMode; method: string };

/** Data flows between components; crossings are computed by the server (FR-064, FR-065). */
@Component({
  selector: 'tm-flows-tab',
  imports: [AiBadgeComponent, FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './flows-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FlowsTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly selectId = input<string | null>(null);
  /** Only the editor, for the diagram: the table is not shown. */
  readonly embedded = input(false);
  /** The diagram's style controls, shown as the drawer's Properties tab. */
  readonly propertiesTpl = input<TemplateRef<unknown> | null>(null);
  readonly drawerTab = signal<'details' | 'properties'>('details');
  readonly relationships = RELATIONSHIPS;
  readonly tri: TriState[] = ['unknown', 'yes', 'no'];
  readonly classifications: Classification[] = ['public', 'internal', 'confidential', 'restricted'];
  readonly label = label;
  readonly crossingOnly = signal(false);

  readonly rows = computed(() =>
    this.crossingOnly() ? this.store.flows().filter(f => f.crossing.crosses) : this.store.flows());
  readonly assetOptions = computed(() => this.store.assets().map(a => ({ id: a.id, label: a.name })));

  private name(f: Flow): string {
    return targetName({ kind: 'flow', id: f.id }, this.store.components(), this.store.flows());
  }

  readonly columns = computed<Column<Flow>[]>(() => [
    { key: 'flow', label: 'Flow', text: f => this.name(f) + (f.label ? ` — ${f.label}` : '') + aiMark(f) },
    { key: 'rel', label: 'Relationship', text: f => label(f.relationship) },
    { key: 'protocol', label: 'Protocol', text: f => f.protocol ?? '—' },
    { key: 'auth', label: 'Authentication', text: f => f.authentication },
    { key: 'enc', label: 'Encrypted', text: f => label(f.encrypted) },
    { key: 'entry', label: 'Entry point', text: f => (f.entryPoint ? '⇥ Yes' : 'No'), sort: f => (f.entryPoint ? 0 : 1) },
    { key: 'crossing', label: 'Trust boundary', text: f => crossingText(f), sort: f => (f.crossing.crosses ? 0 : 1) },
    { key: 'warnings', label: 'Warnings', text: f => f.warnings.map(w => `⚠ ${w.message}`).join('; ') },
  ]);

  readonly editor = new EditorState<Flow, FlowDraft>(
    (d, item) => {
      const { authMode, method, ...input } = d;
      const authentication = authMode === 'method' ? method.trim() || 'unknown' : authMode;
      return this.store.save('flows', {
        ...input, authentication, label: orNull(input.label), protocol: orNull(input.protocol),
        authorizationModel: orNull(input.authorizationModel ?? ''),
      }, item ?? undefined);
    },
    f => this.store.remove('flows', f.id),
  );

  constructor() {
    effect(() => {
      const f = this.store.flows().find(x => x.id === this.selectId());
      if (f) untracked(() => this.edit(f));
    });
  }

  /** A new flow; `sourceId` presets where it starts (the diagram's "Add Data Flow"). */
  add(sourceId?: string): void {
    const cs = this.store.components();
    const source = sourceId ?? cs[0]?.id ?? '';
    const target = cs.find(c => c.id !== source)?.id ?? source;
    this.editor.open(null, {
      sourceId: source, targetId: target, relationship: 'calls',
      label: '', protocol: '', bidirectional: false, authentication: 'unknown', encrypted: 'unknown',
      assetIds: [], authMode: 'unknown', method: '', dataClassification: null,
      authorizationModel: '', entryPoint: false,
    });
  }

  edit(f: Flow): void {
    const a = f.authentication.toLowerCase();
    const authMode: AuthMode = a === 'none' ? 'none' : a === 'unknown' ? 'unknown' : 'method';
    this.editor.open(f, {
      sourceId: f.sourceId, targetId: f.targetId, relationship: f.relationship, label: f.label ?? '',
      protocol: f.protocol ?? '', bidirectional: f.bidirectional, authentication: f.authentication,
      encrypted: f.encrypted, assetIds: [...f.assetIds], authMode,
      method: authMode === 'method' ? f.authentication : '',
      dataClassification: f.dataClassification ?? null, authorizationModel: f.authorizationModel ?? '',
      entryPoint: f.entryPoint ?? false,
    });
  }

  heading(f: Flow | null): string {
    return f ? `Flow ${this.name(f)}` : 'New data flow';
  }
}
