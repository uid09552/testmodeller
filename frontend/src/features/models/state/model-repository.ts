import { Injectable, signal } from '@angular/core';
import { readJson, writeJson } from '../../../core/persistence/local-store';
import { CanvasEdge, CanvasGroup, CanvasNode, ModelStatus, SHAPE_FOR_KIND, SIZE_FOR_SHAPE, StateKind } from './model-editor.store';

/** The full persisted content of one model. */
export interface PersistedModel {
  id: string;
  name: string;
  description: string;
  scenarioDesc: string;
  status: ModelStatus;
  nodes: CanvasNode[];
  edges: CanvasEdge[];
  groups?: CanvasGroup[];
  /** Next test-id counter, so ids are never reused across sessions. */
  testSeq: number;
}

const KEY = 'models';

/**
 * Root-level store of model content, keyed by model id.
 *
 * The per-editor `ModelEditorStore` holds the model being edited; this holds
 * every model so the Explorer and the Test Cases table can read across them,
 * and so content survives a refresh.
 */
@Injectable({ providedIn: 'root' })
export class ModelRepository {
  private readonly models = signal<Record<string, PersistedModel>>(
    readJson<Record<string, PersistedModel>>(KEY) ?? seedDemoModels(),
  );

  /** Reactive view of every stored model. */
  readonly all = this.models.asReadonly();

  get(id: string): PersistedModel | null {
    return this.models()[id] ?? null;
  }

  has(id: string): boolean {
    return id in this.models();
  }

  save(model: PersistedModel): void {
    this.models.update(m => ({ ...m, [model.id]: model }));
    this.flush();
  }

  remove(id: string): void {
    this.models.update(m => {
      const next = { ...m };
      delete next[id];
      return next;
    });
    this.flush();
  }

  private flush(): void {
    writeJson(KEY, this.models());
  }
}

// ── Demo seed ─────────────────────────────────────────────────────────────────
// Matches the ids ExplorerStore seeds, so a first-run workspace has something
// to look at in the editor and the Test Cases table.
function node(
  id: string, label: string, kind: StateKind,
  x: number, y: number, tests: PersistedModel['nodes'][number]['tests'] = [],
): CanvasNode {
  const shape = SHAPE_FOR_KIND[kind];
  const { w, h } = SIZE_FOR_SHAPE[shape];
  return { id, label, kind, x, y, w, h, tests, shape, color: null };
}

function edge(id: string, fromId: string, toId: string, label: string): CanvasEdge {
  return { id, fromId, toId, label, curve: 0 };
}

function seedDemoModels(): Record<string, PersistedModel> {
  const login: PersistedModel = {
    id: 'demo-model-login',
    name: 'Login Flow',
    description: 'Authentication entry point.',
    scenarioDesc:
      'As a user visiting the application, I want to log in with my credentials '
      + 'or continue as a guest so that I can reach the home screen.',
    status: 'draft',
    testSeq: 4,
    nodes: [
      node('ds0', 'Start', 'initial', 80, 160, [
        {
          id: 'dt1', seq: 1, name: 'Application opens on the start screen',
          category: 'feature', polarity: 'positive',
          given: 'the application is not running',
          when:  'the user opens the application',
          then:  'the Start state is shown',
        },
      ]),
      node('ds1', 'Login', 'regular', 300, 80, [
        {
          id: 'dt2', seq: 2, name: 'Valid credentials are accepted',
          category: 'feature', polarity: 'positive',
          given: 'the system is in the "Login" state\nthe account is active',
          when:  'valid credentials are submitted',
          then:  'the Home state is reached',
          implementationUrl: 'https://github.com/example/app/blob/main/e2e/login.spec.ts',
          backlogUrl: 'https://example.atlassian.net/browse/TM-104',
        },
        {
          id: 'dt3', seq: 3, name: 'Blank password is rejected',
          category: 'unit', polarity: 'negative',
          given: 'the system is in the "Login" state',
          when:  'the password field is left blank and submitted',
          then:  'a validation message is shown\nthe Login state is retained',
        },
      ]),
      node('ds2', 'Guest', 'regular', 300, 240),
      node('ds3', 'Home', 'final', 540, 160),
    ],
    edges: [
      edge('de0', 'ds0', 'ds1', 'login'),
      edge('de1', 'ds0', 'ds2', 'guest'),
      edge('de2', 'ds1', 'ds3', 'success'),
      edge('de3', 'ds2', 'ds3', 'enter'),
    ],
  };

  const registration: PersistedModel = {
    id: 'demo-model-registration',
    name: 'Registration Flow',
    description: '',
    scenarioDesc: '',
    status: 'draft',
    testSeq: 2,
    nodes: [
      node('rs0', 'Start', 'initial', 80, 140),
      node('rs1', 'Form', 'regular', 300, 140, [
        {
          id: 'rt1', seq: 1, name: 'Duplicate email is rejected',
          category: 'integration', polarity: 'negative',
          given: 'an account already exists for the email',
          when:  'the registration form is submitted with that email',
          then:  'an "email already in use" error is shown',
          backlogUrl: 'https://example.atlassian.net/browse/TM-117',
        },
      ]),
      node('rs2', 'Confirmed', 'final', 540, 140),
    ],
    edges: [
      edge('re0', 'rs0', 'rs1', 'begin'),
      edge('re1', 'rs1', 'rs2', 'submit'),
    ],
  };

  return { [login.id]: login, [registration.id]: registration };
}
