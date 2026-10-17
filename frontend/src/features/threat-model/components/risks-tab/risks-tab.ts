import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RiskThreshold, RiskTreatment, Threat, ThreatInput } from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { threatInput } from '../../state/inputs';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import {
  download, IMPACT, label, levelText, LIKELIHOOD, riskRegisterCsv,
} from '../../state/threat-model-view';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { Cell, RiskMatrixComponent } from '../risk-matrix/risk-matrix';

type Basis = 'inherent' | 'current';
interface RiskDraft {
  base: ThreatInput;
  il: number | null; ii: number | null; cl: number | null; ci: number | null;
  treatment: RiskTreatment;
  riskOwner: string; dueDate: string; acceptanceRationale: string; acceptedBy: string;
}

/** Likelihood × impact ratings, treatment, matrix and register (FR-070, FR-071). */
@Component({
  selector: 'tm-risks-tab',
  imports: [FormsModule, EntityTableComponent, EditorDrawerComponent, RiskMatrixComponent],
  templateUrl: './risks-tab.html',
  styleUrl: './risks-tab.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RisksTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly basis = signal<Basis>('current');
  readonly cell = signal<{ likelihood: number; impact: number } | null>(null);
  readonly scale = [1, 2, 3, 4, 5];
  readonly likelihoodNames = LIKELIHOOD;
  readonly impactNames = IMPACT;
  readonly treatments: RiskTreatment[] = ['undecided', 'mitigate', 'avoid', 'transfer', 'accept'];
  readonly label = label;

  /** Open and accepted threats: the ones a risk register is about. */
  private readonly live = computed(() =>
    this.store.threats().filter(t => t.status === 'open' || t.status === 'accepted'));

  readonly cells = computed<Cell[]>(() => {
    const m = new Map<string, number>();
    for (const t of this.live()) {
      const r = this.basis() === 'inherent' ? t.inherent : t.current;
      if (r) m.set(`${r.likelihood}:${r.impact}`, (m.get(`${r.likelihood}:${r.impact}`) ?? 0) + 1);
    }
    return [...m].map(([k, count]) => {
      const [likelihood, impact] = k.split(':').map(Number);
      return { likelihood, impact, count };
    });
  });

  readonly rows = computed(() => {
    const c = this.cell();
    return this.live().filter(t => {
      if (!c) return true;
      const r = this.basis() === 'inherent' ? t.inherent : t.current;
      return !!r && r.likelihood === c.likelihood && r.impact === c.impact;
    });
  });

  readonly columns = computed<Column<Threat>[]>(() => [
    { key: 'title', label: 'Threat', text: t => `${t.displayId} ${t.title}` },
    { key: 'inherent', label: 'Inherent', text: t => (t.inherentScore ? `${t.inherentScore} ${levelText(t.inherentLevel)}` : 'Not assessed'), sort: t => t.inherentScore ?? -1 },
    { key: 'current', label: 'Current', text: t => (t.currentScore ? `${t.currentScore} ${levelText(t.currentLevel)}` : 'Not assessed'), sort: t => t.currentScore ?? -1 },
    { key: 'treatment', label: 'Treatment', text: t => label(t.treatment) },
    { key: 'owner', label: 'Owner', text: t => t.riskOwner ?? '—' },
    { key: 'due', label: 'Due', text: t => (t.dueDate ?? '—') + (t.flags.includes('overdue') ? ' ⚠ Overdue' : ''), sort: t => t.dueDate ?? '9999' },
  ]);

  readonly editor = new EditorState<Threat, RiskDraft>(
    (d, item) => {
      const rating = (l: number | null, i: number | null) => (l && i ? { likelihood: l, impact: i } : null);
      return this.store.save('threats', {
        ...d.base,
        inherent: rating(d.il, d.ii), current: rating(d.cl, d.ci), treatment: d.treatment,
        riskOwner: orNull(d.riskOwner), dueDate: orNull(d.dueDate),
        acceptanceRationale: orNull(d.acceptanceRationale), acceptedBy: orNull(d.acceptedBy),
      }, item ?? undefined);
    },
    t => this.store.remove('threats', t.id),
  );

  // Thresholds editor.
  readonly editingThresholds = signal<RiskThreshold[] | null>(null);
  readonly thresholdError = signal<string | null>(null);

  setBasis(b: Basis): void {
    this.basis.set(b);
    this.cell.set(null);
  }

  pick(c: { likelihood: number; impact: number }): void {
    const s = this.cell();
    this.cell.set(s && s.likelihood === c.likelihood && s.impact === c.impact ? null : c);
  }

  edit(t: Threat): void {
    this.editor.open(t, {
      base: threatInput(t),
      il: t.inherent?.likelihood ?? null, ii: t.inherent?.impact ?? null,
      cl: t.current?.likelihood ?? null, ci: t.current?.impact ?? null,
      treatment: t.treatment, riskOwner: t.riskOwner ?? '', dueDate: t.dueDate ?? '',
      acceptanceRationale: t.acceptanceRationale ?? '', acceptedBy: t.acceptedBy ?? '',
    });
  }

  exportCsv(): void {
    download('risk-register.csv', riskRegisterCsv(this.rows()), 'text/csv');
  }

  editThresholds(): void {
    this.thresholdError.set(null);
    this.editingThresholds.set(this.store.model()!.thresholds.map(t => ({ ...t })));
  }

  async saveThresholds(): Promise<void> {
    const m = this.store.model()!;
    try {
      await this.store.saveModel({
        name: m.name, scope: m.scope, objectives: m.objectives, assumptions: m.assumptions,
        exclusions: m.exclusions, thresholds: this.editingThresholds()!,
      });
      this.editingThresholds.set(null);
    } catch (e) {
      this.thresholdError.set(errorMessage(e));
    }
  }
}
