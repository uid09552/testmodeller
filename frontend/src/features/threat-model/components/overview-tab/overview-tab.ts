import { ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ThreatModelInput } from '../../../../core/api/threat-model-api';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import { label, LEVEL_ORDER, levelText } from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';
import { RiskMatrixComponent } from '../risk-matrix/risk-matrix';

/** Scope of the threat model and a summary of what is in it (FR-076). */
@Component({
  selector: 'tm-overview-tab',
  imports: [FormsModule, RiskMatrixComponent],
  templateUrl: './overview-tab.html',
  styleUrl: './overview-tab.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OverviewTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly nav = inject(WorkspaceNav);
  readonly levels = LEVEL_ORDER;
  readonly levelText = levelText;
  readonly label = label;

  readonly draft = signal<ThreatModelInput>({ name: '' });
  readonly saving = signal(false);
  readonly saved = signal(false);
  readonly error = signal<string | null>(null);

  readonly s = computed(() => this.store.summary());
  readonly verified = computed(() => {
    const r = this.s()?.controlsVerifiedRatio;
    return r == null ? '—' : `${Math.round(r * 100)}%`;
  });
  readonly openThreats = computed(() => this.s()?.threatsByStatus['open'] ?? 0);

  constructor() {
    effect(() => {
      const m = this.store.model();
      if (!m) return;
      untracked(() => this.draft.set({
        name: m.name, scope: m.scope ?? '', objectives: m.objectives ?? '',
        assumptions: m.assumptions ?? '', exclusions: m.exclusions ?? '',
      }));
    });
  }

  async save(): Promise<void> {
    const m = this.store.model();
    if (!m) return;
    this.saving.set(true);
    this.error.set(null);
    this.saved.set(false);
    try {
      await this.store.saveModel({ ...this.draft(), thresholds: m.thresholds });
      this.saved.set(true);
    } catch (e) {
      this.error.set(errorMessage(e));
    } finally {
      this.saving.set(false);
    }
  }
}
