import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Baseline, BaselineComparison } from '../../../../core/api/threat-model-api';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import { label, levelText } from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';

/** Immutable snapshots of the threat model and their comparison with it (FR-081). */
@Component({
  selector: 'tm-baselines-tab',
  imports: [FormsModule],
  templateUrl: './baselines-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BaselinesTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly nav = inject(WorkspaceNav);
  readonly label = label;
  readonly levelText = levelText;
  readonly fieldText: Record<string, string | undefined> = {
    status: 'status', inherentLevel: 'inherent level', currentLevel: 'current level',
    residualLevel: 'residual level', treatment: 'treatment',
  };

  readonly name = signal('');
  readonly note = signal('');
  readonly message = signal<string | null>(null);
  readonly comparing = signal<{ baseline: Baseline; result: BaselineComparison } | null>(null);
  readonly levels = ['low', 'moderate', 'high', 'critical', 'not-assessed'];

  async create(): Promise<void> {
    this.message.set(null);
    try {
      await this.store.createBaseline({ name: this.name().trim(), note: this.note().trim() || null });
      this.name.set('');
      this.note.set('');
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  open(b: Baseline): void {
    this.nav.go('report', { baseline: b.id });
  }

  async compare(b: Baseline): Promise<void> {
    this.message.set(null);
    try {
      this.comparing.set({ baseline: b, result: await this.store.compareBaseline(b.id) });
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }

  async remove(b: Baseline): Promise<void> {
    if (!confirm(`Delete baseline "${b.name}"? This cannot be undone.`)) return;
    this.message.set(null);
    try {
      await this.store.deleteBaseline(b.id);
      if (this.comparing()?.baseline.id === b.id) this.comparing.set(null);
    } catch (e) {
      this.message.set(errorMessage(e));
    }
  }
}
