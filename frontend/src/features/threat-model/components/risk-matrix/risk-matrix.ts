import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { RiskLevel, RiskThreshold } from '../../../../core/api/threat-model-api';
import { IMPACT, LEVEL_TEXT, LIKELIHOOD } from '../../state/threat-model-view';

export interface Cell { likelihood: number; impact: number; count: number }

/**
 * The 5 × 5 likelihood × impact matrix. Each cell names its likelihood,
 * impact, level and count, so it reads without colour; selecting a cell
 * reports it (06-ui.md).
 */
@Component({
  selector: 'tm-risk-matrix',
  templateUrl: './risk-matrix.html',
  styleUrl: './risk-matrix.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RiskMatrixComponent {
  readonly cells = input.required<Cell[]>();
  readonly thresholds = input.required<RiskThreshold[]>();
  readonly selected = input<{ likelihood: number; impact: number } | null>(null);
  readonly caption = input('Risk matrix');
  readonly pick = output<{ likelihood: number; impact: number }>();

  readonly likelihoods = [5, 4, 3, 2, 1];
  readonly impacts = [1, 2, 3, 4, 5];
  readonly likelihoodNames = LIKELIHOOD;
  readonly impactNames = IMPACT;

  private readonly counts = computed(() =>
    new Map(this.cells().map(c => [`${c.likelihood}:${c.impact}`, c.count])));

  count(l: number, i: number): number {
    return this.counts().get(`${l}:${i}`) ?? 0;
  }

  level(l: number, i: number): RiskLevel {
    const s = l * i;
    return this.thresholds().find(t => s >= t.min && s <= t.max)?.level ?? 'critical';
  }

  label(l: number, i: number): string {
    const n = this.count(l, i);
    return `Likelihood ${l} ${LIKELIHOOD[l - 1]}, impact ${i} ${IMPACT[i - 1]}, `
      + `${LEVEL_TEXT[this.level(l, i)].slice(2)}, ${n} ${n === 1 ? 'threat' : 'threats'}`;
  }

  isSelected(l: number, i: number): boolean {
    const s = this.selected();
    return !!s && s.likelihood === l && s.impact === i;
  }
}
