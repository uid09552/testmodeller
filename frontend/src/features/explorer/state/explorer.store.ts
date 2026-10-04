import { computed, effect, Injectable, signal } from '@angular/core';
import { readJson, writeJson } from '../../../core/persistence/local-store';

export interface ExplorerModel {
  id: string;
  name: string;
  status: 'draft' | 'review' | 'approved';
  states: number;
  transitions: number;
}

export interface ExplorerFeature {
  id: string;
  name: string;
  description: string;
  models: ExplorerModel[];
  expanded: boolean;
}

export interface ExplorerComponent {
  id: string;
  name: string;
  features: ExplorerFeature[];
  expanded: boolean;
}

export interface ExplorerProject {
  id: string;
  name: string;
  components: ExplorerComponent[];
  expanded: boolean;
}

function uuid(): string { return crypto.randomUUID(); }

const KEY = 'explorer';

/** Stable ids so ModelRepository can seed content for the demo models. */
export const DEMO_LOGIN_MODEL_ID = 'demo-model-login';
export const DEMO_REG_MODEL_ID   = 'demo-model-registration';

@Injectable({ providedIn: 'root' })
export class ExplorerStore {
  readonly projects = signal<ExplorerProject[]>([]);

  // ── Computed ────────────────────────────────────────────────────────────────
  readonly totalModels = computed(() =>
    this.projects().flatMap(p => p.components).flatMap(c => c.features)
      .reduce((s, f) => s + f.models.length, 0),
  );

  constructor() {
    const stored = readJson<ExplorerProject[]>(KEY);
    if (stored?.length) {
      // Tolerate trees stored before `models` existed on features.
      this.projects.set(stored.map(p => ({
        ...p,
        components: p.components.map(c => ({
          ...c,
          features: c.features.map(f => ({ ...f, models: f.models ?? [], expanded: f.expanded ?? true })),
        })),
      })));
    } else {
      this.loadDemo();
    }
    // Persist on every change.
    effect(() => writeJson(KEY, this.projects()));
  }

  /** Flattened tree with full path, for cross-cutting views. */
  readonly modelPaths = computed(() =>
    this.projects().flatMap(p =>
      p.components.flatMap(c =>
        c.features.flatMap(f =>
          f.models.map(m => ({
            projectId: p.id,     projectName: p.name,
            componentId: c.id,   componentName: c.name,
            featureId: f.id,     featureName: f.name,
            model: m,
          })),
        ),
      ),
    ),
  );

  /** Keep the tree's cached counts in step with edited model content. */
  syncModelStats(modelId: string, patch: { name?: string; states?: number; transitions?: number }): void {
    this.projects.update(ps => ps.map(p => ({
      ...p,
      components: p.components.map(c => ({
        ...c,
        features: c.features.map(f => ({
          ...f,
          models: f.models.map(m => m.id === modelId ? { ...m, ...patch } : m),
        })),
      })),
    })));
  }

  // ── Project mutations ──────────────────────────────────────────────────────
  addProject(name: string): ExplorerProject {
    const p: ExplorerProject = { id: uuid(), name, components: [], expanded: true };
    this.projects.update(ps => [...ps, p]);
    return p;
  }

  renameProject(id: string, name: string): void {
    this.projects.update(ps => ps.map(p => p.id === id ? { ...p, name } : p));
  }

  deleteProject(id: string): void {
    this.projects.update(ps => ps.filter(p => p.id !== id));
  }

  toggleProject(id: string): void {
    this.projects.update(ps => ps.map(p => p.id === id ? { ...p, expanded: !p.expanded } : p));
  }

  // ── Component mutations ────────────────────────────────────────────────────
  addComponent(projectId: string, name: string): ExplorerComponent {
    const c: ExplorerComponent = { id: uuid(), name, features: [], expanded: true };
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? { ...p, components: [...p.components, c] } : p,
    ));
    return c;
  }

  renameComponent(projectId: string, componentId: string, name: string): void {
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c => c.id === componentId ? { ...c, name } : c),
      } : p,
    ));
  }

  deleteComponent(projectId: string, componentId: string): void {
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.filter(c => c.id !== componentId),
      } : p,
    ));
  }

  toggleComponent(projectId: string, componentId: string): void {
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c =>
          c.id === componentId ? { ...c, expanded: !c.expanded } : c,
        ),
      } : p,
    ));
  }

  // ── Feature mutations ──────────────────────────────────────────────────────
  addFeature(projectId: string, componentId: string, name: string): ExplorerFeature {
    const f: ExplorerFeature = { id: uuid(), name, description: '', models: [], expanded: true };
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c =>
          c.id === componentId ? { ...c, features: [...c.features, f] } : c,
        ),
      } : p,
    ));
    return f;
  }

  renameFeature(projectId: string, componentId: string, featureId: string, name: string): void {
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c =>
          c.id === componentId ? {
            ...c,
            features: c.features.map(f => f.id === featureId ? { ...f, name } : f),
          } : c,
        ),
      } : p,
    ));
  }

  deleteFeature(projectId: string, componentId: string, featureId: string): void {
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c =>
          c.id === componentId ? {
            ...c,
            features: c.features.filter(f => f.id !== featureId),
          } : c,
        ),
      } : p,
    ));
  }

  toggleFeature(projectId: string, componentId: string, featureId: string): void {
    this.mapFeature(projectId, componentId, featureId, f => ({ ...f, expanded: !f.expanded }));
  }

  // ── Model mutations ────────────────────────────────────────────────────────
  addModel(
    projectId: string, componentId: string, featureId: string, name: string,
    fixedId?: string,
  ): ExplorerModel {
    const m: ExplorerModel = {
      id: fixedId ?? uuid(), name, status: 'draft', states: 0, transitions: 0,
    };
    this.mapFeature(projectId, componentId, featureId,
      f => ({ ...f, models: [...f.models, m], expanded: true }));
    return m;
  }

  renameModel(projectId: string, componentId: string, featureId: string, modelId: string, name: string): void {
    this.mapFeature(projectId, componentId, featureId,
      f => ({ ...f, models: f.models.map(m => m.id === modelId ? { ...m, name } : m) }));
  }

  deleteModel(projectId: string, componentId: string, featureId: string, modelId: string): void {
    this.mapFeature(projectId, componentId, featureId,
      f => ({ ...f, models: f.models.filter(m => m.id !== modelId) }));
  }

  /** Shared structural update for a single feature. */
  private mapFeature(
    projectId: string, componentId: string, featureId: string,
    fn: (f: ExplorerFeature) => ExplorerFeature,
  ): void {
    this.projects.update(ps => ps.map(p =>
      p.id !== projectId ? p : {
        ...p,
        components: p.components.map(c =>
          c.id !== componentId ? c : {
            ...c,
            features: c.features.map(f => f.id === featureId ? fn(f) : f),
          },
        ),
      },
    ));
  }

  // ── Demo seed ──────────────────────────────────────────────────────────────
  loadDemo(): void {
    const proj = this.addProject('My Project');
    const authComp = this.addComponent(proj.id, 'Auth Flow');
    const loginFeat = this.addFeature(proj.id, authComp.id, 'Login');
    this.addModel(proj.id, authComp.id, loginFeat.id, 'Login Flow', DEMO_LOGIN_MODEL_ID);
    const regFeat = this.addFeature(proj.id, authComp.id, 'Registration');
    this.addModel(proj.id, authComp.id, regFeat.id, 'Registration Flow', DEMO_REG_MODEL_ID);
    this.addFeature(proj.id, authComp.id, 'Password Reset');
    const checkoutComp = this.addComponent(proj.id, 'Checkout');
    this.addFeature(proj.id, checkoutComp.id, 'Cart');
    this.addFeature(proj.id, checkoutComp.id, 'Payment');
    const proj2 = this.addProject('Mobile App');
    const onboardComp = this.addComponent(proj2.id, 'Onboarding');
    this.addFeature(proj2.id, onboardComp.id, 'Welcome Screen');
    this.addFeature(proj2.id, onboardComp.id, 'Profile Setup');
    proj2.expanded = false;
    this.projects.update(ps => ps.map(p => p.id === proj2.id ? { ...p, expanded: false } : p));
  }
}
