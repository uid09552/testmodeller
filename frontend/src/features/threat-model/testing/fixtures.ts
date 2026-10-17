/** Builders and an in-memory API for the threat modeling specs. */
import { ApiError } from '../../../core/api/api-error';
import {
  ArchComponent, Asset, CollectionName, Collections, Control, Flow, SecurityTest, Threat,
  ThreatModel, ThreatModelSummary, Zone, Baseline, BaselineComparison, BaselineDetail, Evidence,
  EvidenceInput, TmContextPreview, TmJob, TmProposal,
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
    exclusions: null, thresholds: THRESHOLDS, layout: null, layoutVersion: 0, aiEnabled: false, ...over,
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
    classification: null, dataLocation: null, retention: null, encryptionAtRest: 'unknown',
    componentIds: [], flowIds: [], warnings: [], ...over,
  };
}

export function flow(id: string, sourceId: string, targetId: string, over: Partial<Flow> = {}): Flow {
  return {
    ...audit(id), sourceId, targetId, relationship: 'calls', label: null, protocol: null,
    bidirectional: false, authentication: 'unknown', encrypted: 'unknown', assetIds: [],
    dataClassification: null, authorizationModel: null, entryPoint: false,
    crossing: { crosses: false, direction: null, fromTrust: null, toTrust: null }, warnings: [], ...over,
  };
}

export function threat(id: string, n: number, over: Partial<Threat> = {}): Threat {
  return {
    ...audit(id), displayId: `THR-${n}`, title: `Threat ${n}`, category: 'spoofing', scenario: null,
    attackerObjective: null, preconditions: null, consequences: null, targets: [], assetIds: [],
    controlIds: [], status: 'open', dismissReason: null, inherent: null, current: null,
    residual: null, threatSource: null, detection: null, existingMitigations: null, confidence: null,
    treatment: 'undecided', riskOwner: null, dueDate: null, acceptanceRationale: null,
    acceptedBy: null, acceptanceExpires: null, acceptedOn: null, origin: 'manual', explanation: null,
    inherentScore: null, currentScore: null, inherentLevel: null, currentLevel: null,
    residualScore: null, residualLevel: null, effectiveStatus: over.status ?? 'open',
    affectedBoundaries: [], flags: [], residualFacts: [], ...over,
  };
}

export function control(id: string, n: number, over: Partial<Control> = {}): Control {
  return {
    ...audit(id), displayId: `CTL-${n}`, name: `Control ${n}`, description: null, type: 'preventive',
    category: 'identity-access', status: 'planned', owner: null, acceptanceCriteria: null,
    reviewBy: null, componentIds: [], threatIds: [], verifiedAt: null, securityTestIds: [],
    evidenceCount: 0, flags: [], ...over,
  };
}

export function test(id: string, n: number, over: Partial<SecurityTest> = {}): SecurityTest {
  return {
    ...audit(id), displayId: `SEC-${n}`, title: `Test ${n}`, objective: null, preconditions: [],
    steps: [], type: 'other', status: 'draft', threatIds: [], controlIds: [], origin: 'manual',
    templateId: null, templateVersion: null, stale: null, lastResult: null, ...over,
  };
}

export function proposal(id: string, kind: TmProposal['kind'], payload: Record<string, unknown>, over: Partial<TmProposal> = {}): TmProposal {
  return {
    id, threatModelId: 'tm1', kind, status: 'pending', payload, rationale: 'because', source: 'fake-model',
    promptVersion: 'v1', targetIds: [], resultingEntityIds: [], rejectReason: null,
    createdAt: '2026-10-17T10:00:00Z', ...over,
  };
}

export function summary(over: Partial<ThreatModelSummary> = {}): ThreatModelSummary {
  return {
    components: 0, assets: 0, flows: 0, crossings: 0, threatsByStatus: {}, openByLevel: {},
    inherentMatrix: [], currentMatrix: [], controlsVerifiedRatio: null, threatsWithoutControl: [],
    controlsWithoutTest: [], warningCount: 0, overdueCount: 0, residualMatrix: [], completeness: [],
    expiringAcceptances: [], expiredAcceptances: 0, verificationOverdue: 0, ...over,
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

  proposalList: TmProposal[] = [];
  /** What `tmRequest` answers; a job by default. */
  requestResult: TmJob | TmContextPreview = { id: 'job1', status: 'queued' };
  /** What a context preview answers. */
  previewResult: TmContextPreview = {
    context: '{"threatModel":{"name":"Shop"}}', truncated: false, provider: 'openai-compatible',
    endpointHost: 'llm.example.org',
  };
  /** The job `awaitJob` reports. */
  jobResult: TmJob = { id: 'job1', status: 'succeeded', proposalIds: [] };

  async tmProposals(_: string, status?: string): Promise<TmProposal[]> {
    return this.proposalList.filter(p => !status || p.status === status);
  }

  async tmRequest(_: string, req: unknown): Promise<TmJob | TmContextPreview> {
    if ((req as { previewContext?: boolean }).previewContext) return this.previewResult;
    this.calls.push({ op: 'tmRequest', args: [req] });
    this.fail();
    return this.requestResult;
  }

  async awaitJob(): Promise<TmJob> {
    return this.jobResult;
  }

  async tmGetProposal(_: string, id: string): Promise<TmProposal> {
    return this.proposalList.find(p => p.id === id)!;
  }

  async tmAccept(_: string, id: string, payload?: unknown): Promise<TmProposal> {
    this.calls.push({ op: 'tmAccept', args: [id, payload] });
    this.fail();
    const i = this.proposalList.findIndex(p => p.id === id);
    this.proposalList[i] = { ...this.proposalList[i], status: 'accepted' };
    return this.proposalList[i];
  }

  async tmReject(_: string, id: string, reason?: string): Promise<TmProposal> {
    this.calls.push({ op: 'tmReject', args: [id, reason] });
    const i = this.proposalList.findIndex(p => p.id === id);
    this.proposalList[i] = { ...this.proposalList[i], status: 'rejected', rejectReason: reason ?? null };
    return this.proposalList[i];
  }

  evidence: Evidence[] = [];
  baselineList: Baseline[] = [];
  baselineDetail: BaselineDetail | null = null;
  comparison: BaselineComparison = {
    added: [], removed: [], changed: [], controlsChanged: [], levelsBaseline: {}, levelsLive: {},
  };

  async controlEvidence(_: string, controlId: string): Promise<Evidence[]> {
    return this.evidence.filter(e => e.ownerKind === 'control' && e.ownerId === controlId);
  }

  async addControlEvidence(_: string, controlId: string, input: EvidenceInput): Promise<Evidence> {
    this.calls.push({ op: 'addControlEvidence', args: [controlId, input] });
    this.fail();
    const e: Evidence = { ...audit(`ev${this.seq++}`), ...input, ownerKind: 'control', ownerId: controlId };
    this.evidence.push(e);
    // Replace, as a reload from the server does: an editor holding the old object is stale.
    this.data.controls = this.data.controls.map(c =>
      c.id === controlId ? { ...c, evidenceCount: c.evidenceCount + 1 } : c);
    return e;
  }

  async deleteEvidence(_: string, evidenceId: string): Promise<void> {
    this.calls.push({ op: 'deleteEvidence', args: [evidenceId] });
    const i = this.evidence.findIndex(e => e.id === evidenceId);
    const [e] = this.evidence.splice(i, 1);
    this.data.controls = this.data.controls.map(c => {
      if (c.id !== e.ownerId) return c;
      const n = c.evidenceCount - 1;
      return { ...c, evidenceCount: n, status: n === 0 && c.status === 'verified' ? 'implemented' : c.status };
    });
  }

  async baselines(): Promise<Baseline[]> {
    return [...this.baselineList];
  }

  async createBaseline(_: string, input: { name: string; note?: string | null }): Promise<Baseline> {
    this.calls.push({ op: 'createBaseline', args: [input] });
    this.fail();
    const b: Baseline = {
      id: `bl${this.seq++}`, name: input.name, note: input.note ?? null, createdAt: '2026-10-17T10:00:00Z',
      createdBy: 'tester', threatCount: this.data.threats.length, controlCount: this.data.controls.length,
      schemaVersion: 1,
    };
    this.baselineList.unshift(b);
    return b;
  }

  async baseline(): Promise<BaselineDetail> {
    return this.baselineDetail!;
  }

  async deleteBaseline(_: string, id: string): Promise<void> {
    this.calls.push({ op: 'deleteBaseline', args: [id] });
    this.baselineList = this.baselineList.filter(b => b.id !== id);
  }

  async compareBaseline(): Promise<BaselineComparison> {
    return this.comparison;
  }

  writes(op: string): Call[] {
    return this.calls.filter(c => c.op === op);
  }
}
