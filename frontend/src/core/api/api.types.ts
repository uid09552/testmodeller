/**
 * Wire types for `/api/v1`, mirroring docs/specification/openapi.yaml.
 *
 * The contract is the source of truth; these are hand-written because the
 * generator is not wired up yet. Keep the names identical to the schema names
 * so a reviewer can diff them against the YAML.
 */

/** RFC 7807 problem details. */
export interface Problem {
  type?: string;
  title?: string;
  status?: number;
  detail?: string;
  errors?: { field: string; message: string }[];
}

export type JobStatus = 'queued' | 'running' | 'succeeded' | 'failed';

export interface Job {
  id: string;
  status: JobStatus;
  proposalIds?: string[];
  error?: Problem;
}

export type ProposalKind =
  | 'model'
  | 'states-and-transitions'
  | 'test-cases'
  | 'feature-description';

export type ProposalStatus = 'pending' | 'accepted' | 'rejected';

export interface ProposalRequest {
  kind: ProposalKind;
  featureId: string;
  modelId?: string;
  /** Free-text instruction; the contract caps it at 8000 characters. */
  prompt?: string;
  count?: number;
}

export interface Proposal {
  id: string;
  projectId: string;
  featureId?: string;
  modelId?: string;
  kind: ProposalKind;
  status: ProposalStatus;
  /** Shape depends on `kind`; see the payload types below. */
  payload: unknown;
  rationale?: string;
  /** Model id of the LLM that produced it. */
  source?: string;
  resultingEntityId?: string;
  createdAt: string;
}

// ── Proposal payloads, by kind ────────────────────────────────────────────────

export interface TestStepInput {
  action: string;
  expected: string;
}

/** Payload of a `test-cases` proposal: one test case per proposal. */
export interface TestCaseInput {
  name: string;
  description?: string;
  preconditions?: string;
  priority?: 'low' | 'medium' | 'high';
  status?: 'draft' | 'approved' | 'deprecated';
  tags?: string[];
  /** Optional http(s) link to the implementation. */
  implementationUrl?: string;
  /** Optional http(s) link to the backlog item (the requirement). */
  backlogUrl?: string;
  steps: TestStepInput[];
  assignments?: AssignmentInput[];
}

/** Payload of a `feature-description` proposal. */
export interface FeatureDescriptionPayload {
  scenarioDescription: string;
}

/** The API's state kinds; the canvas has more (see `ai-chat.store`). */
export type ApiStateKind = 'initial' | 'normal' | 'final';

export interface StateInput {
  id?: string;
  name: string;
  description?: string;
  kind: ApiStateKind;
  position?: { x: number; y: number };
}

export interface TransitionInput {
  id?: string;
  from: string;
  to: string;
  event: string;
  guard?: string;
  action?: string;
  expected?: string;
}

/** The contract's model status; the editor has more (see model-mapping). */
export type ApiModelStatus = 'draft' | 'ready';

export interface AssignmentInput {
  modelId: string;
  stateId?: string;
  transitionId?: string;
  stepOrder?: number;
}

/** A stored test case, as `GET /models/{id}/test-cases` returns it. */
/** An imported execution result (`TestResult` in openapi.yaml). */
export type ResultStatus = 'passed' | 'failed' | 'skipped' | 'error';
export interface TestResult {
  id: string;
  testCaseId: string;
  runId: string;
  status: ResultStatus;
  durationMs?: number;
  message?: string;
  executedAt: string;
  source: string;
}

export interface TestCase extends TestCaseInput {
  id: string;
  version: number;
  featureId: string;
  assignments: (AssignmentInput & { testCaseId: string })[];
  /** Read-only: most recent imported result. */
  lastResult?: TestResult;
}

/** Payload of a `states-and-transitions` proposal. */
export interface ElementsPayload {
  states: StateInput[];
  transitions: TransitionInput[];
}

/** Payload of a `model` proposal, and the body of a model save. */
export interface ModelInput {
  name: string;
  description?: string;
  status?: ApiModelStatus;
  states?: StateInput[];
  transitions?: TransitionInput[];
}

// ── Settings ──────────────────────────────────────────────────────────────────

/** The providers the contract allows. */
export type AiProvider = 'openai-compatible' | 'anthropic' | 'local' | 'none';

export interface AiSettings {
  provider?: AiProvider;
  baseUrl?: string;
  model?: string;
  /** Read-only: the key itself is never returned. */
  secretConfigured?: boolean;
  maxTokensPerRequest?: number;
}

