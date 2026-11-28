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
  /** Threats of the project's threat model it covers (ADR 0016); omit to keep the links. */
  threatIds?: string[];
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
  /** Requirements covered: backlog URLs or short keys (at most 50). */
  requirements?: string[];
}

export interface TransitionInput {
  id?: string;
  from: string;
  to: string;
  event: string;
  guard?: string;
  action?: string;
  expected?: string;
  /** Relative likelihood in a `random` walk (0..1000, default 1). */
  weight?: number;
  /** Requirements covered: backlog URLs or short keys (at most 50). */
  requirements?: string[];
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
  /** How it was created (`Origin` in openapi.yaml). */
  origin?: 'manual' | 'generated' | 'ai';
  /** The model a generated test case came from. */
  generatedFromModelId?: string;
}

/** Payload of a `states-and-transitions` proposal. */
export interface ElementsPayload {
  states: StateInput[];
  transitions: TransitionInput[];
}

/** A model variable (`Variable` in openapi.yaml). The editor carries these through unchanged. */
export interface Variable {
  name: string;
  type: 'boolean' | 'integer' | 'string';
  initial?: unknown;
}

/** The editor's layout (`ModelLayout`, ADR 0010): opaque to the backend. */
export type ModelLayoutDoc = Record<string, unknown>;

/** Payload of a `model` proposal, and the body of a model save. */
export interface ModelInput {
  name: string;
  description?: string;
  status?: ApiModelStatus;
  variables?: Variable[];
  states?: StateInput[];
  transitions?: TransitionInput[];
  /** Absent keeps the stored layout; `null` clears it. */
  layout?: ModelLayoutDoc | null;
}

// ── Settings ──────────────────────────────────────────────────────────────────

/** The providers the contract allows. */
export type AiProvider = 'openai-compatible' | 'anthropic' | 'local' | 'none';

/** A personal API key for the MCP endpoint (tag `ApiKeys`); never holds the secret. */
export interface ApiKey {
  id: string;
  name: string;
  /** First characters of the key, to recognise it. */
  prefix: string;
  role: 'User' | 'Editor';
  /** Set for an upload key: the one project whose test results it may post (ADR 0018). */
  projectId?: string | null;
  createdAt: string;
  expiresAt?: string | null;
  lastUsedAt?: string | null;
}

export interface ApiKeyInput {
  name: string;
  expiresInDays?: number;
  /** Makes an upload key for this project; needs the Editor role. */
  projectId?: string;
}

/** The response to creating a key: the only time `key` is ever returned. */
export interface ApiKeyCreated extends ApiKey {
  key: string;
}

export interface AiSettings {
  provider?: AiProvider;
  baseUrl?: string;
  model?: string;
  /** Read-only: the key itself is never returned. */
  secretConfigured?: boolean;
  maxTokensPerRequest?: number;
}

