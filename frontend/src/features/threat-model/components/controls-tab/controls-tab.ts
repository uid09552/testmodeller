import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  Control, ControlCategory, ControlInput, ControlStatus, ControlType, Evidence, EvidenceInput,
  EvidenceKind,
} from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { controlInput } from '../../state/inputs';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import { CONTROL_FLAG_TEXT, label, aiMark } from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';
import { AiBadgeComponent } from '../ai-badge/ai-badge';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

type Gap = '' | 'no-threats' | 'no-tests';

/** Security controls and what they cover (FR-072). */
@Component({
  selector: 'tm-controls-tab',
  imports: [AiBadgeComponent, FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
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
  readonly flagText = CONTROL_FLAG_TEXT;
  readonly evidenceKinds: EvidenceKind[] = ['document', 'test-report', 'scan', 'configuration', 'review', 'other'];
  /** Evidence of the control being edited, loaded when its editor opens. */
  readonly evidence = signal<Evidence[]>([]);
  readonly evidenceError = signal<string | null>(null);
  readonly newEvidence = signal<EvidenceInput>(this.blankEvidence());
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
    { key: 'name', label: 'Control', text: c => `${c.displayId} ${c.name}${aiMark(c)}`, sort: c => Number(c.displayId.slice(4)) },
    { key: 'type', label: 'Type', text: c => label(c.type) },
    { key: 'category', label: 'Category', text: c => label(c.category) },
    { key: 'status', label: 'Status', text: c => label(c.status) + (c.verifiedAt ? ` (${c.verifiedAt.slice(0, 10)})` : '') },
    { key: 'evidence', label: 'Evidence', text: c => `${c.evidenceCount}`, sort: c => c.evidenceCount },
    { key: 'flags', label: 'Flags', text: c => c.flags.map(f => `⚠ ${CONTROL_FLAG_TEXT[f]}`).join('; ') },
    { key: 'threats', label: 'Threats', text: c => this.threatIds(c) },
    { key: 'tests', label: 'Security tests', text: c => `${c.securityTestIds.length}`, sort: c => c.securityTestIds.length },
  ]);

  readonly editor = new EditorState<Control, ControlInput>(
    (d, item) => this.store.save('controls', {
      ...d, description: orNull(d.description), owner: orNull(d.owner),
      acceptanceCriteria: orNull(d.acceptanceCriteria), reviewBy: orNull(d.reviewBy),
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
      owner: '', acceptanceCriteria: '', reviewBy: '', componentIds: [], threatIds: [],
    });
    this.evidence.set([]);
  }

  edit(c: Control): void {
    this.editor.open(c, { ...controlInput(c), reviewBy: c.reviewBy ?? '' });
    this.evidenceError.set(null);
    this.newEvidence.set(this.blankEvidence());
    void this.loadEvidence(c.id);
  }

  private blankEvidence(): EvidenceInput {
    return { kind: 'test-report', title: '', link: '', note: '', evidenceDate: new Date().toISOString().slice(0, 10), reviewer: '' };
  }

  private async loadEvidence(controlId: string): Promise<void> {
    try {
      this.evidence.set(await this.store.controlEvidence(controlId));
    } catch (e) {
      this.evidenceError.set(errorMessage(e));
    }
  }

  /** "Verified" is only offered with evidence (FR-078); the server enforces it too. */
  canVerify(c: Control | null): boolean {
    if (!c) return false;
    // The editor holds the control as it was opened; evidence added since is only in the store.
    const fresh = this.store.controls().find(x => x.id === c.id) ?? c;
    return fresh.evidenceCount > 0 || fresh.status === 'verified';
  }

  async addEvidence(c: Control): Promise<void> {
    const e = this.newEvidence();
    this.evidenceError.set(null);
    try {
      await this.store.addControlEvidence(c.id, {
        ...e, link: orNull(e.link ?? ''), note: orNull(e.note ?? ''), reviewer: orNull(e.reviewer ?? ''),
      });
      this.newEvidence.set(this.blankEvidence());
      await this.loadEvidence(c.id);
    } catch (err) {
      this.evidenceError.set(errorMessage(err));
    }
  }

  async removeEvidence(c: Control, ev: Evidence): Promise<void> {
    this.evidenceError.set(null);
    try {
      await this.store.deleteEvidence(ev.id);
      await this.loadEvidence(c.id);
      const fresh = this.store.controls().find(x => x.id === c.id);
      if (fresh && this.editor.editing()) this.editor.editing()!.draft.status = fresh.status;
    } catch (err) {
      this.evidenceError.set(errorMessage(err));
    }
  }

  patchEvidence(patch: Partial<EvidenceInput>): void {
    this.newEvidence.update(e => ({ ...e, ...patch }));
  }

  testsOf(c: Control) {
    return this.store.tests().filter(t => c.securityTestIds.includes(t.id));
  }
}
