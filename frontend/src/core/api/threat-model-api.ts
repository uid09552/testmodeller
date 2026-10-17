import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom, Observable } from 'rxjs';
import { toApiError } from './api-error';

/**
 * Threat model resources (tag `ThreatModel` in openapi.yaml). See
 * docs/specification/09-threat-modeling.md.
 */

const BASE = '/api/v1';

export type ArchComponentType =
  | 'web-frontend' | 'mobile-app' | 'desktop-client' | 'browser-extension'
  | 'api-gateway' | 'api' | 'message-broker' | 'webhook' | 'external-connector'
  | 'service' | 'worker' | 'scheduler' | 'serverless-function'
  | 'relational-database' | 'object-storage' | 'cache' | 'search-index' | 'file-store' | 'backup'
  | 'identity-provider' | 'authorization-service' | 'secrets-manager' | 'key-management' | 'audit-service'
  | 'cloud-service' | 'container-platform' | 'cicd-pipeline' | 'third-party-saas' | 'library'
  | 'human-user' | 'external-system';
export type ArchLayer =
  | 'client' | 'integration' | 'logic' | 'data' | 'identity-security' | 'platform' | 'external';
export type Criticality = 'low' | 'medium' | 'high' | 'critical';
export type ZoneKind =
  | 'internet' | 'partner' | 'dmz' | 'internal' | 'restricted-data' | 'management'
  | 'identity' | 'cicd' | 'third-party' | 'other';
export type AssetCategory =
  | 'business-data' | 'identity-secrets' | 'software-code' | 'infrastructure'
  | 'security-asset' | 'operational-asset' | 'business-capability';
export type CiaLevel = 'low' | 'medium' | 'high';
export type FlowRelationship =
  | 'calls' | 'sends-data' | 'reads-writes' | 'authenticates-through' | 'trusts' | 'depends-on'
  | 'runs-on' | 'publishes-subscribes' | 'replicates-backs-up' | 'deploys';
export type TriState = 'yes' | 'no' | 'unknown';
export type StrideCategory =
  | 'spoofing' | 'tampering' | 'repudiation' | 'information-disclosure'
  | 'denial-of-service' | 'elevation-of-privilege';
export type ThreatStatus = 'candidate' | 'open' | 'mitigated' | 'accepted' | 'dismissed';
export type RiskLevel = 'low' | 'moderate' | 'high' | 'critical';
export type RiskTreatment = 'undecided' | 'mitigate' | 'avoid' | 'transfer' | 'accept';
export type ThreatFlag =
  | 'no-control' | 'planned-controls-only' | 'unsupported-reduction' | 'overdue' | 'failing-test'
  | 'residual-above-current' | 'unsupported-residual' | 'acceptance-expired';
export type ControlFlag = 'verification-overdue' | 'verified-without-evidence';
export type Confidence = 'low' | 'medium' | 'high';
export type Classification = 'public' | 'internal' | 'confidential' | 'restricted';
export type EvidenceKind = 'document' | 'test-report' | 'scan' | 'configuration' | 'review' | 'other';
export type ControlType = 'preventive' | 'detective' | 'corrective' | 'compensating';
export type ControlCategory =
  | 'identity-access' | 'data-protection' | 'application-network' | 'detection-response'
  | 'secure-development';
export type ControlStatus = 'planned' | 'implemented' | 'verified';
export type SecurityTestType =
  | 'negative-authorization' | 'authentication' | 'input-validation' | 'transport-security'
  | 'configuration-review' | 'resilience' | 'logging-audit' | 'other';
export type SecurityTestStatus = 'draft' | 'ready' | 'obsolete';
export type SecurityTestOutcome = 'passed' | 'failed' | 'not-run';

export interface Audit {
  id: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface Warning { code: string; message: string }

export interface RiskThreshold { level: RiskLevel; min: number; max: number }

export interface ThreatModelInput {
  name: string;
  scope?: string | null;
  objectives?: string | null;
  assumptions?: string | null;
  exclusions?: string | null;
  thresholds?: RiskThreshold[];
  /** Absent keeps the stored value. */
  aiEnabled?: boolean;
}
export interface ThreatModel extends Audit {
  projectId: string;
  name: string;
  scope: string | null;
  objectives: string | null;
  assumptions: string | null;
  exclusions: string | null;
  thresholds: RiskThreshold[];
  layout: Record<string, unknown> | null;
  layoutVersion: number;
  aiEnabled: boolean;
}

/** How an architecture entity or control came into existence. */
export type AiOrigin = 'manual' | 'ai';
export interface Provenance {
  origin?: AiOrigin;
  /** Model id of the LLM when the origin is ai. */
  source?: string | null;
}

export interface ZoneInput {
  name: string;
  description?: string | null;
  kind: ZoneKind;
  trustLevel: number;
}
export type Zone = Audit & Required<ZoneInput> & Provenance;

export interface ArchComponentInput {
  name: string;
  description?: string | null;
  type: ArchComponentType;
  owner?: string | null;
  technology?: string | null;
  criticality?: Criticality | null;
  internetFacing: boolean;
  zoneId?: string | null;
  linkedComponentId?: string | null;
  linkedFeatureId?: string | null;
  assetIds: string[];
}
export interface ArchComponent extends Audit, ArchComponentInput, Provenance {
  layer: ArchLayer;
  warnings: Warning[];
}

export interface AssetInput {
  name: string;
  description?: string | null;
  category: AssetCategory;
  owner?: string | null;
  confidentiality?: CiaLevel | null;
  integrity?: CiaLevel | null;
  availability?: CiaLevel | null;
  personalData: boolean;
  classification?: Classification | null;
  dataLocation?: string | null;
  retention?: string | null;
  encryptionAtRest?: TriState;
}
export interface Asset extends Audit, AssetInput, Provenance {
  componentIds: string[];
  flowIds: string[];
  warnings: Warning[];
}

export interface FlowInput {
  sourceId: string;
  targetId: string;
  relationship: FlowRelationship;
  label?: string | null;
  protocol?: string | null;
  bidirectional: boolean;
  authentication: string;
  encrypted: TriState;
  assetIds: string[];
  dataClassification?: Classification | null;
  authorizationModel?: string | null;
  entryPoint?: boolean;
}
export interface Crossing {
  crosses: boolean;
  direction: 'into-more-trusted' | 'into-less-trusted' | null;
  fromTrust: number | null;
  toTrust: number | null;
}
export interface Flow extends Audit, FlowInput, Provenance {
  crossing: Crossing;
  warnings: Warning[];
}

export interface Rating { likelihood: number; impact: number }
export interface ThreatTarget { kind: 'component' | 'flow'; id: string }
export interface RuleExplanation {
  ruleId: string;
  ruleVersion: number;
  ruleName: string;
  facts: string[];
  missing: string[];
  recommendations: string[];
}
export interface ThreatInput {
  title: string;
  category: StrideCategory;
  scenario?: string | null;
  attackerObjective?: string | null;
  preconditions?: string | null;
  consequences?: string | null;
  targets: ThreatTarget[];
  assetIds: string[];
  controlIds: string[];
  status: ThreatStatus;
  dismissReason?: string | null;
  inherent?: Rating | null;
  current?: Rating | null;
  residual?: Rating | null;
  threatSource?: string | null;
  detection?: string | null;
  existingMitigations?: string | null;
  confidence?: Confidence | null;
  treatment: RiskTreatment;
  riskOwner?: string | null;
  dueDate?: string | null;
  acceptanceRationale?: string | null;
  acceptedBy?: string | null;
  acceptanceExpires?: string | null;
}
export interface Threat extends Audit, ThreatInput {
  displayId: string;
  origin: 'manual' | 'rule' | 'ai';
  source?: string | null;
  explanation: RuleExplanation | null;
  inherentScore: number | null;
  currentScore: number | null;
  inherentLevel: RiskLevel | null;
  currentLevel: RiskLevel | null;
  residualScore: number | null;
  residualLevel: RiskLevel | null;
  acceptedOn: string | null;
  /** An accepted threat whose acceptance lapsed reads as open. */
  effectiveStatus: ThreatStatus;
  affectedBoundaries: Crossing[];
  flags: ThreatFlag[];
  residualFacts: string[];
}

export interface ControlInput {
  name: string;
  description?: string | null;
  type: ControlType;
  category: ControlCategory;
  status: ControlStatus;
  owner?: string | null;
  acceptanceCriteria?: string | null;
  reviewBy?: string | null;
  componentIds: string[];
  threatIds: string[];
}
export interface Control extends Audit, ControlInput, Provenance {
  displayId: string;
  verifiedAt: string | null;
  securityTestIds: string[];
  evidenceCount: number;
  flags: ControlFlag[];
}

export interface EvidenceInput {
  kind: EvidenceKind;
  title: string;
  link?: string | null;
  note?: string | null;
  evidenceDate: string;
  reviewer?: string | null;
}
export interface Evidence extends Audit, EvidenceInput {
  ownerKind: 'control' | 'test-result';
  ownerId: string;
}

export interface BaselineInput { name: string; note?: string | null }
export interface Baseline {
  id: string;
  name: string;
  note: string | null;
  createdAt: string;
  createdBy: string | null;
  threatCount: number;
  controlCount: number;
  schemaVersion: number;
}
export interface ThreatModelSnapshot {
  threatModel: ThreatModel;
  zones: Zone[];
  components: ArchComponent[];
  assets: Asset[];
  flows: Flow[];
  threats: Threat[];
  controls: Control[];
  securityTests: SecurityTest[];
  evidence: Evidence[];
}
export interface BaselineDetail extends Baseline { snapshot: ThreatModelSnapshot }
export interface ComparedThreat {
  id: string;
  displayId: string;
  title: string;
  changes: { field: string; before: string | null; after: string | null }[];
}
export interface BaselineComparison {
  added: ComparedThreat[];
  removed: ComparedThreat[];
  changed: ComparedThreat[];
  controlsChanged: { id: string; displayId: string; before: string; after: string }[];
  levelsBaseline: Record<string, number>;
  levelsLive: Record<string, number>;
}

export interface SecurityTestStep { action: string; expected: string }
export interface SecurityTestInput {
  title: string;
  objective?: string | null;
  preconditions: string[];
  steps: SecurityTestStep[];
  type: SecurityTestType;
  status: SecurityTestStatus;
  threatIds: string[];
  controlIds: string[];
}
export interface SecurityTestResult {
  id: string;
  securityTestId: string;
  outcome: SecurityTestOutcome;
  note: string | null;
  executedAt: string;
}
export interface SecurityTest extends Audit, SecurityTestInput {
  displayId: string;
  origin: 'manual' | 'generated' | 'ai';
  source?: string | null;
  templateId: string | null;
  templateVersion: number | null;
  stale: string | null;
  lastResult: SecurityTestResult | null;
}

export interface MatrixCell { likelihood: number; impact: number; count: number }
export interface RefLabel { id: string; label: string }
export interface ThreatModelSummary {
  components: number;
  assets: number;
  flows: number;
  crossings: number;
  threatsByStatus: Record<string, number>;
  openByLevel: Record<string, number>;
  inherentMatrix: MatrixCell[];
  currentMatrix: MatrixCell[];
  controlsVerifiedRatio: number | null;
  threatsWithoutControl: RefLabel[];
  controlsWithoutTest: RefLabel[];
  warningCount: number;
  overdueCount: number;
  residualMatrix: MatrixCell[];
  completeness: CompletenessFinding[];
  expiringAcceptances: RefLabel[];
  expiredAcceptances: number;
  verificationOverdue: number;
}
export interface CompletenessFinding {
  code: string;
  message: string;
  entityKind: string;
  entityId: string;
}

export type TmProposalKind =
  | 'tm-architecture' | 'tm-threats' | 'tm-ratings' | 'tm-controls' | 'tm-security-tests' | 'tm-review';
export type ProposalStatus = 'pending' | 'accepted' | 'rejected';
export interface TmProposalRequest {
  kind: TmProposalKind;
  prompt?: string;
  targetIds?: string[];
  count?: number;
  previewContext?: boolean;
}
export interface TmContextPreview {
  context: string;
  truncated: boolean;
  provider: string;
  endpointHost: string | null;
}
export interface TmProposal {
  id: string;
  threatModelId: string;
  kind: TmProposalKind;
  status: ProposalStatus;
  payload: Record<string, unknown>;
  rationale: string | null;
  source: string | null;
  promptVersion: string | null;
  targetIds: string[];
  resultingEntityIds: string[];
  rejectReason: string | null;
  createdAt: string;
}
export interface TmJob {
  id: string;
  status: 'queued' | 'running' | 'succeeded' | 'failed';
  proposalIds?: string[];
  error?: { detail?: string };
}

export interface SuggestThreatsResult { rulesetVersion: number; created: Threat[]; skipped: number }
export interface GenerateSecurityTestsResult { created: SecurityTest[]; skipped: number }

/** Collections under `/threat-models/{id}` with plain CRUD. */
export interface Collections {
  zones: { input: ZoneInput; item: Zone };
  components: { input: ArchComponentInput; item: ArchComponent };
  assets: { input: AssetInput; item: Asset };
  flows: { input: FlowInput; item: Flow };
  threats: { input: ThreatInput; item: Threat };
  controls: { input: ControlInput; item: Control };
  'security-tests': { input: SecurityTestInput; item: SecurityTest };
}
export type CollectionName = keyof Collections;

@Injectable({ providedIn: 'root' })
export class ThreatModelApi {
  private readonly http = inject(HttpClient);

  private async call<T>(o: Observable<T>): Promise<T> {
    try {
      return await firstValueFrom(o);
    } catch (e) {
      throw toApiError(e);
    }
  }

  private ifMatch(version: number) {
    return { headers: { 'If-Match': String(version) } };
  }

  ofProject(projectId: string): Promise<ThreatModel> {
    return this.call(this.http.get<ThreatModel>(`${BASE}/projects/${projectId}/threat-model`));
  }

  create(projectId: string, input: ThreatModelInput): Promise<ThreatModel> {
    return this.call(this.http.post<ThreatModel>(`${BASE}/projects/${projectId}/threat-model`, input));
  }

  update(id: string, version: number, input: ThreatModelInput): Promise<ThreatModel> {
    return this.call(this.http.put<ThreatModel>(`${BASE}/threat-models/${id}`, input, this.ifMatch(version)));
  }

  saveLayout(id: string, layoutVersion: number, layout: Record<string, unknown> | null): Promise<{ layoutVersion: number }> {
    return this.call(this.http.put<{ layoutVersion: number }>(
      `${BASE}/threat-models/${id}/layout`, layout, this.ifMatch(layoutVersion)));
  }

  deleteModel(id: string): Promise<void> {
    return this.call(this.http.delete<void>(`${BASE}/threat-models/${id}`));
  }

  summary(id: string): Promise<ThreatModelSummary> {
    return this.call(this.http.get<ThreatModelSummary>(`${BASE}/threat-models/${id}/summary`));
  }

  list<C extends CollectionName>(id: string, c: C): Promise<Collections[C]['item'][]> {
    return this.call(this.http.get<Collections[C]['item'][]>(`${BASE}/threat-models/${id}/${c}`));
  }

  createItem<C extends CollectionName>(id: string, c: C, input: Collections[C]['input']): Promise<Collections[C]['item']> {
    return this.call(this.http.post<Collections[C]['item']>(`${BASE}/threat-models/${id}/${c}`, input));
  }

  updateItem<C extends CollectionName>(
    id: string, c: C, itemId: string, version: number, input: Collections[C]['input'],
  ): Promise<Collections[C]['item']> {
    return this.call(this.http.put<Collections[C]['item']>(
      `${BASE}/threat-models/${id}/${c}/${itemId}`, input, this.ifMatch(version)));
  }

  deleteItem(id: string, c: CollectionName, itemId: string): Promise<void> {
    return this.call(this.http.delete<void>(`${BASE}/threat-models/${id}/${c}/${itemId}`));
  }

  suggestThreats(id: string): Promise<SuggestThreatsResult> {
    return this.call(this.http.post<SuggestThreatsResult>(`${BASE}/threat-models/${id}/threats/suggest`, {}));
  }

  bulkThreatStatus(id: string, ids: string[], status: 'open' | 'dismissed', reason?: string): Promise<Threat[]> {
    return this.call(this.http.post<Threat[]>(
      `${BASE}/threat-models/${id}/threats/bulk-status`, { ids, status, reason }));
  }

  generateTests(id: string, threatIds?: string[]): Promise<GenerateSecurityTestsResult> {
    return this.call(this.http.post<GenerateSecurityTestsResult>(
      `${BASE}/threat-models/${id}/security-tests/generate`, threatIds ? { threatIds } : {}));
  }

  bulkTestStatus(id: string, ids: string[], status: SecurityTestStatus): Promise<SecurityTest[]> {
    return this.call(this.http.post<SecurityTest[]>(
      `${BASE}/threat-models/${id}/security-tests/bulk-status`, { ids, status }));
  }

  tmProposals(id: string, status?: ProposalStatus): Promise<TmProposal[]> {
    const q = status ? `?status=${status}` : '';
    return this.call(this.http.get<TmProposal[]>(`${BASE}/threat-models/${id}/ai/proposals${q}`));
  }

  /** Starts a job (202), or returns the context that would be sent when `previewContext` is set. */
  tmRequest(id: string, req: TmProposalRequest): Promise<TmJob | TmContextPreview> {
    return this.call(this.http.post<TmJob | TmContextPreview>(`${BASE}/threat-models/${id}/ai/proposals`, req));
  }

  tmGetProposal(id: string, proposalId: string): Promise<TmProposal> {
    return this.call(this.http.get<TmProposal>(`${BASE}/threat-models/${id}/ai/proposals/${proposalId}`));
  }

  tmAccept(id: string, proposalId: string, payload?: unknown): Promise<TmProposal> {
    return this.call(this.http.post<TmProposal>(
      `${BASE}/threat-models/${id}/ai/proposals/${proposalId}/accept`, payload === undefined ? {} : { payload }));
  }

  tmReject(id: string, proposalId: string, reason?: string): Promise<TmProposal> {
    return this.call(this.http.post<TmProposal>(
      `${BASE}/threat-models/${id}/ai/proposals/${proposalId}/reject`, reason ? { reason } : {}));
  }

  getJob(jobId: string): Promise<TmJob> {
    return this.call(this.http.get<TmJob>(`${BASE}/jobs/${jobId}`));
  }

  /** Polls a job until it finishes or `timeoutMs` has passed. */
  async awaitJob(jobId: string, timeoutMs = 180_000, pollMs = 1_200): Promise<TmJob> {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      const job = await this.getJob(jobId);
      if (job.status === 'succeeded' || job.status === 'failed') return job;
      if (Date.now() > deadline) {
        return { id: jobId, status: 'failed', error: { detail: 'The AI job did not finish in time.' } };
      }
      await new Promise(r => setTimeout(r, pollMs));
    }
  }

  controlEvidence(id: string, controlId: string): Promise<Evidence[]> {
    return this.call(this.http.get<Evidence[]>(`${BASE}/threat-models/${id}/controls/${controlId}/evidence`));
  }

  addControlEvidence(id: string, controlId: string, input: EvidenceInput): Promise<Evidence> {
    return this.call(this.http.post<Evidence>(`${BASE}/threat-models/${id}/controls/${controlId}/evidence`, input));
  }

  resultEvidence(id: string, testId: string, resultId: string): Promise<Evidence[]> {
    return this.call(this.http.get<Evidence[]>(
      `${BASE}/threat-models/${id}/security-tests/${testId}/results/${resultId}/evidence`));
  }

  addResultEvidence(id: string, testId: string, resultId: string, input: EvidenceInput): Promise<Evidence> {
    return this.call(this.http.post<Evidence>(
      `${BASE}/threat-models/${id}/security-tests/${testId}/results/${resultId}/evidence`, input));
  }

  deleteEvidence(id: string, evidenceId: string): Promise<void> {
    return this.call(this.http.delete<void>(`${BASE}/threat-models/${id}/evidence/${evidenceId}`));
  }

  baselines(id: string): Promise<Baseline[]> {
    return this.call(this.http.get<Baseline[]>(`${BASE}/threat-models/${id}/baselines`));
  }

  createBaseline(id: string, input: BaselineInput): Promise<Baseline> {
    return this.call(this.http.post<Baseline>(`${BASE}/threat-models/${id}/baselines`, input));
  }

  baseline(id: string, baselineId: string): Promise<BaselineDetail> {
    return this.call(this.http.get<BaselineDetail>(`${BASE}/threat-models/${id}/baselines/${baselineId}`));
  }

  deleteBaseline(id: string, baselineId: string): Promise<void> {
    return this.call(this.http.delete<void>(`${BASE}/threat-models/${id}/baselines/${baselineId}`));
  }

  compareBaseline(id: string, baselineId: string): Promise<BaselineComparison> {
    return this.call(this.http.get<BaselineComparison>(
      `${BASE}/threat-models/${id}/baselines/${baselineId}/compare`));
  }

  recordResult(
    id: string, testId: string, input: { outcome: SecurityTestOutcome; note?: string; executedAt?: string },
  ): Promise<SecurityTestResult> {
    return this.call(this.http.post<SecurityTestResult>(
      `${BASE}/threat-models/${id}/security-tests/${testId}/results`, input));
  }
}
