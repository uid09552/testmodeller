import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked, viewChild,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  RiskLevel, StrideCategory, Threat, ThreatInput, ThreatStatus,
} from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { threatInput } from '../../state/inputs';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import { FLAG_TEXT, label, levelText, targetName } from '../../state/threat-model-view';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

export const CATEGORIES: StrideCategory[] = [
  'spoofing', 'tampering', 'repudiation', 'information-disclosure', 'denial-of-service',
  'elevation-of-privilege',
];
const STATUSES: ThreatStatus[] = ['candidate', 'open', 'mitigated', 'accepted', 'dismissed'];

type ThreatDraft = ThreatInput & { targetIds: string[] };

/** Threats, manual or suggested by STRIDE rules (FR-068, FR-069). */
@Component({
  selector: 'tm-threats-tab',
  imports: [FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './threats-tab.html',
  styleUrl: './threats-tab.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ThreatsTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly selectId = input<string | null>(null);
  /** Shows only threats on this component or flow (from the diagram). */
  readonly targetId = input<string | null>(null);
  readonly categories = CATEGORIES;
  readonly statuses = STATUSES;
  readonly levels: RiskLevel[] = ['low', 'moderate', 'high', 'critical'];
  readonly label = label;
  readonly flagText = FLAG_TEXT;

  readonly status = signal('');
  readonly category = signal('');
  readonly origin = signal('');
  readonly level = signal('');
  readonly target = signal('');
  readonly checked = signal<string[]>([]);
  readonly message = signal<string | null>(null);
  private readonly table = viewChild<EntityTableComponent<Threat>>(EntityTableComponent);

  readonly targetOptions = computed(() => [
    ...this.store.components().map(c => ({ id: c.id, label: c.name })),
    ...this.store.flows().map(f => ({ id: f.id, label: this.tname('flow', f.id) })),
  ]);
  readonly assetOptions = computed(() => this.store.assets().map(a => ({ id: a.id, label: a.name })));
  readonly controlOptions = computed(() =>
    this.store.controls().map(c => ({ id: c.id, label: `${c.displayId} ${c.name}` })));

  private tname(kind: 'component' | 'flow', id: string): string {
    return targetName({ kind, id }, this.store.components(), this.store.flows());
  }

  readonly rows = computed(() => this.store.threats().filter(t =>
    (!this.status() || t.status === this.status())
    && (!this.category() || t.category === this.category())
    && (!this.origin() || t.origin === this.origin())
    && (!this.level() || (t.currentLevel ?? 'none') === this.level())
    && (!this.target() || t.targets.some(x => x.id === this.target()))));

  readonly columns = computed<Column<Threat>[]>(() => [
    { key: 'title', label: 'Threat', text: t => `${t.displayId} ${t.title}`, sort: t => Number(t.displayId.slice(4)) },
    { key: 'category', label: 'Category', text: t => label(t.category) },
    { key: 'targets', label: 'Targets', text: t => t.targets.map(x => this.tname(x.kind, x.id)).join(', ') },
    { key: 'status', label: 'Status', text: t => (t.status === 'candidate' ? '◌ Candidate' : label(t.status)) },
    { key: 'origin', label: 'Origin', text: t => (t.origin === 'rule' ? 'Rule' : 'Manual') },
    { key: 'risk', label: 'Current risk', text: t => levelText(t.currentLevel), sort: t => t.currentScore ?? -1 },
    { key: 'flags', label: 'Flags', text: t => t.flags.map(f => FLAG_TEXT[f]).join('; ') },
  ]);

  readonly editor = new EditorState<Threat, ThreatDraft>(
    (d, item) => {
      const { targetIds, ...input } = d;
      const flowIds = new Set(this.store.flows().map(f => f.id));
      return this.store.save('threats', {
        ...input,
        targets: targetIds.map(id => ({ kind: flowIds.has(id) ? 'flow' : 'component', id })),
        scenario: orNull(input.scenario), attackerObjective: orNull(input.attackerObjective),
        preconditions: orNull(input.preconditions), consequences: orNull(input.consequences),
        dismissReason: orNull(input.dismissReason),
      }, item ?? undefined);
    },
    t => this.store.remove('threats', t.id),
  );

  constructor() {
    effect(() => {
      const t = this.store.threats().find(x => x.id === this.selectId());
      if (t) untracked(() => this.edit(t));
    });
    effect(() => {
      const id = this.targetId();
      untracked(() => this.target.set(id ?? ''));
    });
  }

  add(): void {
    this.editor.open(null, {
      title: '', category: 'spoofing', scenario: '', attackerObjective: '', preconditions: '',
      consequences: '', targets: [], targetIds: this.target() ? [this.target()] : [], assetIds: [],
      controlIds: [], status: 'open', dismissReason: '', inherent: null, current: null,
      treatment: 'undecided',
    });
  }

  edit(t: Threat): void {
    this.editor.open(t, { ...threatInput(t), targetIds: t.targets.map(x => x.id) });
  }

  async suggest(): Promise<void> {
    this.message.set(null);
    try {
      const r = await this.store.suggestThreats();
      this.message.set(r.created.length
        ? `${r.created.length} candidate threat(s) suggested. Review and confirm or dismiss them.`
        : `No new candidates: every rule match already has a threat${r.skipped ? ` (${r.skipped} skipped)` : ''}.`);
      if (r.created.length) this.status.set('candidate');
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  async bulk(status: 'open' | 'dismissed'): Promise<void> {
    const ids = this.checked();
    if (!ids.length) return;
    let reason: string | undefined;
    if (status === 'dismissed') {
      reason = prompt(`Why are these ${ids.length} threat(s) dismissed?`)?.trim();
      if (!reason) return;
    }
    try {
      await this.store.bulkThreatStatus(ids, status, reason);
      this.table()?.clearChecked();
      this.message.set(`${ids.length} threat(s) ${status === 'open' ? 'confirmed' : 'dismissed'}.`);
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  /** "Add as control": a planned control linked to the threat. */
  async addControl(t: Threat, name: string): Promise<void> {
    try {
      await this.store.save('controls', {
        name, type: 'preventive', category: 'application-network', status: 'planned',
        componentIds: t.targets.filter(x => x.kind === 'component').map(x => x.id), threatIds: [t.id],
      });
      const fresh = this.store.threats().find(x => x.id === t.id);
      if (fresh) this.edit(fresh);
    } catch (e) {
      this.editor.error.set(errorMessage(e));
    }
  }

  hasControlNamed(t: Threat, name: string): boolean {
    return this.store.controls().some(c => c.name === name && c.threatIds.includes(t.id));
  }
}
