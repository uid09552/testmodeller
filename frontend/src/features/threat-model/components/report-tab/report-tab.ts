import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked,
  ViewEncapsulation,
} from '@angular/core';
import {
  Evidence, ThreatModelSnapshot,
} from '../../../../core/api/threat-model-api';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import {
  evidenceOf, flaggedThreats, inRegister, MATRIX_BASES, matrixCells, REPORT_SECTIONS, ReportInput,
  reportMarkdown, warningLines,
} from '../../state/report';
import {
  aiMark, CONTROL_FLAG_TEXT, download, label, levelText, targetName,
} from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';
import { RiskMatrixComponent } from '../risk-matrix/risk-matrix';

/**
 * The assessment report of the live model or of a baseline (FR-082). A
 * baseline is shown here read-only: the banner names it, and nothing on the
 * page edits the model.
 */
@Component({
  selector: 'tm-report-tab',
  imports: [RiskMatrixComponent],
  templateUrl: './report-tab.html',
  styleUrl: './report-tab.scss',
  // The print rules must reach the application chrome around the report.
  encapsulation: ViewEncapsulation.None,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReportTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly nav = inject(WorkspaceNav);
  /** Id of a baseline; the live model when null. */
  readonly baselineId = input<string | null>(null);

  readonly sections = REPORT_SECTIONS;
  readonly bases = MATRIX_BASES;
  readonly label = label;
  readonly aiMark = aiMark;
  readonly levelText = levelText;
  readonly flagged = flaggedThreats;
  readonly inRegister = inRegister;
  readonly warnings = warningLines;
  readonly matrix = matrixCells;
  readonly controlFlagText = CONTROL_FLAG_TEXT;

  readonly report = signal<ReportInput | null>(null);
  readonly error = signal<string | null>(null);
  private readonly liveEvidence = signal<Evidence[]>([]);
  private generation = 0;

  readonly snapshot = computed(() => this.report()?.snapshot ?? null);

  constructor() {
    effect(() => {
      const id = this.baselineId();
      // Rebuilt when the live model changes.
      const model = this.store.model();
      this.store.controls();
      this.store.threats();
      if (!model) return;
      untracked(() => void this.build(id));
    });
  }

  private liveSnapshot(evidence: Evidence[]): ThreatModelSnapshot {
    return {
      threatModel: this.store.model()!, zones: this.store.zones(), components: this.store.components(),
      assets: this.store.assets(), flows: this.store.flows(), threats: this.store.threats(),
      controls: this.store.controls(), securityTests: this.store.tests(), evidence,
    };
  }

  private async build(baselineId: string | null): Promise<void> {
    const gen = ++this.generation;
    this.error.set(null);
    const today = new Date().toISOString().slice(0, 10);
    try {
      if (baselineId) {
        const b = await this.store.baseline(baselineId);
        if (gen !== this.generation) return;
        this.report.set({
          snapshot: b.snapshot, generatedOn: today,
          baseline: { name: b.name, createdAt: b.createdAt, note: b.note },
        });
        return;
      }
      const evidence = (await Promise.all(
        this.store.controls().filter(c => c.evidenceCount > 0).map(c => this.store.controlEvidence(c.id)),
      )).flat();
      if (gen !== this.generation) return;
      this.liveEvidence.set(evidence);
      this.report.set({ snapshot: this.liveSnapshot(evidence), generatedOn: today, baseline: null });
    } catch (e) {
      if (gen === this.generation) this.error.set(errorMessage(e));
    }
  }

  flowName(s: ThreatModelSnapshot, id: string): string {
    return targetName({ kind: 'flow', id }, s.components, s.flows);
  }

  zoneName(s: ThreatModelSnapshot, id: string | null | undefined): string {
    return s.zones.find(z => z.id === id)?.name ?? 'no zone';
  }

  evidenceFor(s: ThreatModelSnapshot, id: string): Evidence[] {
    return evidenceOf(s, id);
  }

  print(): void {
    window.print();
  }

  exportMarkdown(): void {
    const r = this.report();
    if (!r) return;
    const name = (r.baseline?.name ?? r.snapshot.threatModel.name).replace(/[^\w.-]+/g, '-');
    download(`tra-${name}.md`, reportMarkdown(r), 'text/markdown');
  }
}
