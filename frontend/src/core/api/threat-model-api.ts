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
  | 'no-control' | 'planned-controls-only' | 'unsupported-reduction' | 'overdue' | 'failing-test';
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
}

export interface ZoneInput {
  name: string;
  description?: string | null;
  kind: ZoneKind;
  trustLevel: number;
}
export type Zone = Audit & Required<ZoneInput>;

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
export interface ArchComponent extends Audit, ArchComponentInput {
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
}
export interface Asset extends Audit, AssetInput {
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
}
export interface Crossing {
  crosses: boolean;
  direction: 'into-more-trusted' | 'into-less-trusted' | null;
  fromTrust: number | null;
  toTrust: number | null;
}
export interface Flow extends Audit, FlowInput {
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
  treatment: RiskTreatment;
  riskOwner?: string | null;
  dueDate?: string | null;
  acceptanceRationale?: string | null;
  acceptedBy?: string | null;
}
export interface Threat extends Audit, ThreatInput {
  displayId: string;
  origin: 'manual' | 'rule';
  explanation: RuleExplanation | null;
  inherentScore: number | null;
  currentScore: number | null;
  inherentLevel: RiskLevel | null;
  currentLevel: RiskLevel | null;
  flags: ThreatFlag[];
}

export interface ControlInput {
  name: string;
  description?: string | null;
  type: ControlType;
  category: ControlCategory;
  status: ControlStatus;
  owner?: string | null;
  acceptanceCriteria?: string | null;
  componentIds: string[];
  threatIds: string[];
}
export interface Control extends Audit, ControlInput {
  displayId: string;
  verifiedAt: string | null;
  securityTestIds: string[];
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
  origin: 'manual' | 'generated';
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

  recordResult(
    id: string, testId: string, input: { outcome: SecurityTestOutcome; note?: string; executedAt?: string },
  ): Promise<SecurityTestResult> {
    return this.call(this.http.post<SecurityTestResult>(
      `${BASE}/threat-models/${id}/security-tests/${testId}/results`, input));
  }
}
