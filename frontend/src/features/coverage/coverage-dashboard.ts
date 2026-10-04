import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';

interface CoverageRow {
  model: string;
  component: string;
  states: number;
  statesCovered: number;
  transitions: number;
  transitionsCovered: number;
}

@Component({
  selector: 'tm-coverage-dashboard',
  templateUrl: './coverage-dashboard.html',
  styleUrl: './coverage-dashboard.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CoverageDashboardComponent {
  // Placeholder data until GET /coverage is wired up.
  readonly rows = signal<CoverageRow[]>([
    { model: 'Login Flow',        component: 'Auth Flow', states: 4, statesCovered: 4, transitions: 5,  transitionsCovered: 5 },
    { model: 'Registration Flow', component: 'Auth Flow', states: 6, statesCovered: 5, transitions: 8,  transitionsCovered: 6 },
    { model: 'Reset Password',    component: 'Auth Flow', states: 3, statesCovered: 2, transitions: 3,  transitionsCovered: 1 },
    { model: 'Cart Checkout',     component: 'Checkout',  states: 7, statesCovered: 3, transitions: 11, transitionsCovered: 4 },
    { model: 'Payment',           component: 'Checkout',  states: 5, statesCovered: 0, transitions: 7,  transitionsCovered: 0 },
  ]);

  readonly totalStates      = computed(() => this.rows().reduce((s, r) => s + r.states, 0));
  readonly coveredStates    = computed(() => this.rows().reduce((s, r) => s + r.statesCovered, 0));
  readonly totalTransitions = computed(() => this.rows().reduce((s, r) => s + r.transitions, 0));
  readonly coveredTransitions = computed(() => this.rows().reduce((s, r) => s + r.transitionsCovered, 0));

  readonly statePct = computed(() =>
    this.totalStates() ? Math.round((this.coveredStates() / this.totalStates()) * 100) : 0);
  readonly transitionPct = computed(() =>
    this.totalTransitions() ? Math.round((this.coveredTransitions() / this.totalTransitions()) * 100) : 0);

  readonly fullyCovered = computed(() =>
    this.rows().filter(r => r.statesCovered === r.states && r.transitionsCovered === r.transitions).length);

  readonly uncovered = computed(() =>
    this.rows().filter(r => r.statesCovered === 0).length);

  pct(covered: number, total: number): number {
    return total ? Math.round((covered / total) * 100) : 0;
  }

  /** Colour band for a coverage percentage. */
  band(p: number): 'high' | 'mid' | 'low' {
    if (p >= 80) return 'high';
    if (p >= 40) return 'mid';
    return 'low';
  }

  /** Circumference offset for the donut gauge (r = 38). */
  dashOffset(p: number): number {
    const c = 2 * Math.PI * 38;
    return c - (c * p) / 100;
  }

  readonly circumference = 2 * Math.PI * 38;
}
