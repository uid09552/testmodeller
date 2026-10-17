/** Builders and an in-memory API for the threat modeling specs. */
import { ApiError } from '../../../core/api/api-error';
import {
  ArchComponent, Asset, CollectionName, Collections, Control, Flow, SecurityTest, Threat,
  ThreatModel, ThreatModelSummary, Zone,
} from '../../../core/api/threat-model-api';

const audit = (id: string) => ({ id, version: 1, createdAt: '2026-10-01T00:00:00Z', updatedAt: '2026-10-01T00:00:00Z' });

export const THRESHOLDS = [
  { level: 'low' as const, min: 1, max: 4 },
  { level: 'moderate' as const, min: 5, max: 9 },
  { level: 'high' as const, min: 10, max: 16 },
  { level: 'critical' as const, min: 17, max: 25 },
];

export function model(over: Partial<ThreatModel> = {}): ThreatModel {
  return {
    ...audit('tm1'), projectId: 'p1', name: 'Shop', scope: null, objectives: null, assumptions: null,
    exclusions: null, thresholds: THRESHOLDS, layout: null, layoutVersion: 0, ...over,
  };
}

export function zone(id: string, name: string, trustLevel: number): Zone {
  return { ...audit(id), name, description: null, kind: 'internal', trustLevel };
}

export function component(id: string, name: string, over: Partial<ArchComponent> = {}): ArchComponent {
  return {
    ...audit(id), name, description: null, type: 'service', owner: null, technology: null,
    criticality: null, internetFacing: false, zoneId: null, linkedComponentId: null,
    linkedFeatureId: null, assetIds: [], layer: 'logic', warnings: [], ...over,
  };
}

export function asset(id: string, name: string, over: Partial<Asset> = {}): Asset {
  return {
    ...audit(id), name, description: null, category: 'business-data', owner: null,
    confidentiality: 'high', integrity: null, availability: null, personalData: false,
    componentIds: [], flowIds: [], warnings: [], ...over,
  };
}

export function flow(id: string, sourceId: string, targetId: string, over: Partial<Flow> = {}): Flow {
  return {
    ...audit(id), sourceId, targetId, relationship: 'calls', label: null, protocol: null,
    bidirectional: false, authentication: 'unknown', encrypted: 'unknown', assetIds: [],
    crossing: { crosses: false, direction: null, fromTrust: null, toTrust: null }, warnings: [], ...over,
  };
}

export function threat(id: string, n: number, over: Partial<Threat> = {}): Threat {
  return {
    ...audit(id), displayId: `THR-${n}`, title: `Threat ${n}`, category: 'spoofing', scenario: null,
    attackerObjective: null, preconditions: null, consequences: null, targets: [], assetIds: [],
    controlIds: [], status: 'open', dismissReason: null, inherent: null, current: null,
    treatment: 'undecided', riskOwner: null, dueDate: null, acceptanceRationale: null,
    acceptedBy: null, origin: 'manual', explanation: null, inherentScore: null, currentScore: null,
    inherentLevel: null, currentLevel: null, flags: [], ...over,
  };
}

export function control(id: string, n: number, over: Partial<Control> = {}): Control {
  return {
    ...audit(id), displayId: `CTL-${n}`, name: `Control ${n}`, description: null, type: 'preventive',
    category: 'identity-access', status: 'planned', owner: null, acceptanceCriteria: null,
    componentIds: [], threatIds: [], verifiedAt: null, securityTestIds: [], ...over,
  };
}

export function test(id: string, n: number, over: Partial<SecurityTest> = {}): SecurityTest {
  return {
    ...audit(id), displayId: `SEC-${n}`, title: `Test ${n}`, objective: null, preconditions: [],
    steps: [], type: 'other', status: 'draft', threatIds: [], controlIds: [], origin: 'manual',
    templateId: null, templateVersion: null, stale: null, lastResult: null, ...over,
  };
}

export function summary(over: Partial<ThreatModelSummary> = {}): ThreatModelSummary {
  return {
    components: 0, assets: 0, flows: 0, crossings: 0, threatsByStatus: {}, openByLevel: {},
    inherentMatrix: [], currentMatrix: [], controlsVerifiedRatio: null, threatsWithoutControl: [],
    controlsWithoutTest: [], warningCount: 0, overdueCount: 0, ...over,
  };
}

export interface Data {
  zones: Zone[];
  components: ArchComponent[];
  assets: Asset[];
  flows: Flow[];
  threats: Threat[];
  controls: Control[];
  'security-tests': SecurityTest[];
}

export function emptyData(): Data {
  return { zones: [], components: [], assets: [], flows: [], threats: [], controls: [], 'security-tests': [] };
}

/** One recorded write. */
export interface Call { op: string; args: unknown[] }

/**
 * An in-memory `ThreatModelApi`. Items are stored as given (computed fields
 * come from the fixtures), and every write is recorded in `calls`.
 */
export class FakeThreatModelApi {
  data: Data = emptyData();
  model: ThreatModel | null = model();
  summaryValue = summary();
  calls: Call[] = [];
  /** Makes the next write fail with this status. */
  failNext: { status: number; message: string } | null = null;
  private seq = 100;

  private fail(): void {
    const f = this.failNext;
    if (f) {
      this.failNext = null;
      throw new ApiError(f.message, f.status);
    }
  }

  async ofProject(): Promise<ThreatModel> {
    if (!this.model) throw new ApiError('threat model not found', 404);
    return this.model;
  }

  async create(projectId: string, input: { name: string }): Promise<ThreatModel> {
    this.calls.push({ op: 'create', args: [projectId, input] });
    this.model = model({ projectId, name: input.name });
    return this.model;
  }

  async update(id: string, version: number, input: unknown): Promise<ThreatModel> {
    this.calls.push({ op: 'update', args: [id, version, input] });
    this.fail();
    this.model = { ...this.model!, ...(input as object), version: version + 1 };
    return this.model;
  }

  async saveLayout(id: string, layoutVersion: number, layout: unknown): Promise<{ layoutVersion: number }> {
    this.calls.push({ op: 'saveLayout', args: [id, layoutVersion, layout] });
    this.model = { ...this.model!, layout: layout as Record<string, unknown>, layoutVersion: layoutVersion + 1 };
    return { layoutVersion: layoutVersion + 1 };
  }

  async summary(): Promise<ThreatModelSummary> {
    return this.summaryValue;
  }

  async list<C extends CollectionName>(_: string, c: C): Promise<Collections[C]['item'][]> {
    return [...this.data[c]] as Collections[C]['item'][];
  }

  async createItem<C extends CollectionName>(_: string, c: C, input: Collections[C]['input']): Promise<Collections[C]['item']> {
    this.calls.push({ op: 'createItem', args: [c, input] });
    this.fail();
    const id = `new${this.seq++}`;
    const base = { component, flow, threat, control, test } as const;
    const item = c === 'components' ? { ...base.component(id, ''), ...input }
      : c === 'flows' ? { ...base.flow(id, '', ''), ...input }
      : c === 'threats' ? { ...base.threat(id, this.seq), ...input }
      : c === 'controls' ? { ...base.control(id, this.seq), ...input }
      : c === 'security-tests' ? { ...base.test(id, this.seq), ...input }
      : { ...audit(id), ...input };
    (this.data[c] as unknown[]).push(item);
    return item as Collections[C]['item'];
  }

  async updateItem<C extends CollectionName>(
    _: string, c: C, itemId: string, version: number, input: Collections[C]['input'],
  ): Promise<Collections[C]['item']> {
    this.calls.push({ op: 'updateItem', args: [c, itemId, version, input] });
    this.fail();
    const list = this.data[c] as { id: string; version: number }[];
    const i = list.findIndex(x => x.id === itemId);
    list[i] = { ...list[i], ...input, version: version + 1 };
    return list[i] as Collections[C]['item'];
  }

  async deleteItem(_: string, c: CollectionName, itemId: string): Promise<void> {
    this.calls.push({ op: 'deleteItem', args: [c, itemId] });
    (this.data[c] as { id: string }[]).splice((this.data[c] as { id: string }[]).findIndex(x => x.id === itemId), 1);
  }

  suggestResult = { rulesetVersion: 1, created: [] as Threat[], skipped: 0 };
  async suggestThreats() {
    this.calls.push({ op: 'suggestThreats', args: [] });
    this.data.threats.push(...this.suggestResult.created);
    return this.suggestResult;
  }

  async bulkThreatStatus(_: string, ids: string[], status: string, reason?: string) {
    this.calls.push({ op: 'bulkThreatStatus', args: [ids, status, reason] });
    return [];
  }

  generateResult = { created: [] as SecurityTest[], skipped: 0 };
  async generateTests(_: string, threatIds?: string[]) {
    this.calls.push({ op: 'generateTests', args: [threatIds] });
    return this.generateResult;
  }

  async bulkTestStatus(_: string, ids: string[], status: string) {
    this.calls.push({ op: 'bulkTestStatus', args: [ids, status] });
    return [];
  }

  async recordResult(_: string, testId: string, input: unknown) {
    this.calls.push({ op: 'recordResult', args: [testId, input] });
    return { id: 'r1', securityTestId: testId, outcome: 'passed' as const, note: null, executedAt: '2026-10-17T00:00:00Z' };
  }

  writes(op: string): Call[] {
    return this.calls.filter(c => c.op === op);
  }
}
