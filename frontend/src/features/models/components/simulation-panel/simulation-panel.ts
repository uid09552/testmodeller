import {
  ChangeDetectionStrategy, Component, computed, DestroyRef, effect, inject, input, signal, untracked,
} from '@angular/core';
import { ApiError } from '../../../../core/api/api-error';
import { ModelInput, TestCaseInput } from '../../../../core/api/api.types';
import { OrgApi, SimulationStep } from '../../../../core/api/org-api';
import { ModelEditorStore } from '../../state/model-editor.store';
import { testFromApi, toModelInput } from '../../state/model-mapping';
import { ModelPersistenceService } from '../../state/model-persistence';
import { singleLine } from '../../state/node-fit';

/** One step of the trail: where the simulation is, and the transition that led there. */
interface Frame { step: SimulationStep; taken?: string }

/**
 * Step through the model: guards and actions are evaluated by the backend
 * exactly as generation evaluates them, on the graph as it is on screen
 * (unsaved edits included). See docs/specification/06-ui.md#simulation.
 */
@Component({
  selector: 'tm-simulation-panel',
  templateUrl: './simulation-panel.html',
  styleUrl: './simulation-panel.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SimulationPanelComponent {
  private readonly api = inject(OrgApi);
  readonly store = inject(ModelEditorStore);
  private readonly remote = inject(ModelPersistenceService);

  /** The open model and its feature, once loaded. */
  readonly modelId = input<string | null>(null);
  readonly featureId = input<string | null>(null);

  readonly frames = signal<Frame[]>([]);
  readonly busy = signal(false);
  readonly error = signal<string | null>(null);
  readonly issues = signal<string[]>([]);
  readonly savedName = signal<string | null>(null);

  readonly current = computed(() => this.frames().at(-1)?.step ?? null);
  readonly stuck = computed(() => {
    const c = this.current();
    return !!c && !c.final && !c.transitions.some(t => t.enabled);
  });
  readonly variables = computed(() => Object.entries(this.current()?.env ?? {}));
  /** The trail, as "event → state" lines. */
  readonly trail = computed(() => this.frames().slice(1).map(f => ({
    event: this.store.edgeById(f.taken!)?.label ?? '?',
    state: this.stateName(f.step.stateId),
  })));

  constructor() {
    // Mark the current state and its transitions on the canvas.
    effect(() => {
      const c = this.current();
      untracked(() => this.store.simulation.set(c ? {
        currentId: c.stateId,
        enabled: new Set(c.transitions.filter(t => t.enabled).map(t => t.transitionId)),
        blocked: new Set(c.transitions.filter(t => !t.enabled).map(t => t.transitionId)),
      } : null));
    });
    inject(DestroyRef).onDestroy(() => this.store.simulation.set(null));
  }

  stateName(id: string): string { return singleLine(this.store.nodeById(id)?.label ?? '?'); }
  edgeLabel(id: string): string {
    const e = this.store.edgeById(id);
    return e ? `${e.label} → ${this.stateName(e.toId)}` : '?';
  }
  guard(id: string): string | undefined { return this.store.edgeById(id)?.guard; }
  showValue(v: unknown): string { return typeof v === 'string' ? `"${v}"` : String(v); }

  private graph(): ModelInput {
    const { layout: _layout, ...input } = toModelInput(this.store.toPersisted(this.modelId() ?? ''));
    return input;
  }

  private async call(body: { stateId?: string; env?: Record<string, unknown>; take?: string }): Promise<SimulationStep | null> {
    const id = this.modelId();
    if (!id) return null;
    this.busy.set(true);
    this.error.set(null);
    this.issues.set([]);
    try {
      return await this.api.simulate(id, { graph: this.graph(), ...body });
    } catch (e) {
      this.error.set(e instanceof ApiError ? e.message : 'The simulation could not run.');
      this.issues.set(e instanceof ApiError ? (e.problem?.errors ?? []).map(x => x.message) : []);
      return null;
    } finally {
      this.busy.set(false);
    }
  }

  async start(): Promise<void> {
    this.savedName.set(null);
    const step = await this.call({});
    this.frames.set(step ? [{ step }] : []);
  }

  async take(transitionId: string): Promise<void> {
    const c = this.current();
    if (!c) return;
    const step = await this.call({ stateId: c.stateId, env: c.env, take: transitionId });
    if (step) this.frames.update(f => [...f, { step, taken: transitionId }]);
  }

  back(): void { this.frames.update(f => f.length > 1 ? f.slice(0, -1) : f); }

  stop(): void { this.frames.set([]); }

  /** The trail as a manual test case, assigned to its states and transitions in step order. */
  trailInput(): TestCaseInput | null {
    const frames = this.frames();
    const modelId = this.modelId();
    if (frames.length < 2 || !modelId) return null;
    const startId = frames[0].step.stateId;
    const route = [this.stateName(startId), ...frames.slice(1).map(f => this.stateName(f.step.stateId))];
    return {
      name: `Simulation: ${route.join(' → ')}`.slice(0, 300),
      preconditions: `System is in state '${this.stateName(startId)}'`,
      tags: ['feature', 'positive'],
      steps: frames.slice(1).map(f => ({
        action: this.store.edgeById(f.taken!)?.label ?? '?',
        expected: `State '${this.stateName(f.step.stateId)}' is reached`,
      })),
      assignments: [
        { modelId, stateId: startId },
        ...frames.slice(1).flatMap((f, i) => [
          { modelId, transitionId: f.taken!, stepOrder: i + 1 },
          { modelId, stateId: f.step.stateId, stepOrder: i + 1 },
        ]),
      ],
    };
  }

  async saveAsTestCase(): Promise<void> {
    const input = this.trailInput();
    const featureId = this.featureId();
    const modelId = this.modelId();
    if (!input || !featureId || !modelId) return;
    this.busy.set(true);
    try {
      const tc = await this.api.createTestCase(featureId, input);
      const startId = this.frames()[0].step.stateId;
      const test = testFromApi(tc, 0, modelId);
      this.store.adoptTest(startId, test);
      this.remote.adoptTest(test, startId, tc);
      this.savedName.set(tc.name);
    } catch (e) {
      this.error.set(e instanceof ApiError ? e.message : 'The test case could not be saved.');
    } finally {
      this.busy.set(false);
    }
  }
}
