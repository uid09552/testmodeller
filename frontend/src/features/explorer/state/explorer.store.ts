import { computed, inject, Injectable, signal } from '@angular/core';
import { ApiError } from '../../../core/api/api-error';
import { ModelSummary, OrgApi } from '../../../core/api/org-api';

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

/**
 * What the tree has selected, as ids rather than object references: the tree is
 * rebuilt on every edit, so a held reference would go stale.
 */
export type ExplorerSelection =
  | { kind: 'project';   projectId: string }
  | { kind: 'component'; projectId: string; componentId: string }
  | { kind: 'feature';   projectId: string; componentId: string; featureId: string }
  | { kind: 'model';     projectId: string; componentId: string; featureId: string; modelId: string };

/** What the "new item" dialog is about to create. */
export type NewItemKind = 'project' | 'component' | 'feature' | 'model';

/** Request to open the "new item" dialog, with the parent it hangs off. */
export interface NewItemPrompt {
  kind: NewItemKind;
  projectId?: string;
  componentId?: string;
  featureId?: string;
  /** Captured because "+" does not move the selection. */
  featureName?: string;
}

function toExplorerModel(m: ModelSummary): ExplorerModel {
  return {
    id: m.id, name: m.name,
    status: m.status === 'ready' ? 'approved' : 'draft',
    states: m.stateCount, transitions: m.transitionCount,
  };
}

/**
 * The Project > Component > Feature > Model tree, as the backend holds it.
 *
 * Nothing is kept in the browser: the tree is read from the API on start and
 * every change is made through it first, then shown.
 */
@Injectable({ providedIn: 'root' })
export class ExplorerStore {
  private readonly api = inject(OrgApi);

  readonly projects = signal<ExplorerProject[]>([]);
  /** False until the first load has finished. */
  readonly loaded = signal(false);
  /** The last failure, for the UI to show. */
  readonly error = signal<string | null>(null);

  // ── Computed ────────────────────────────────────────────────────────────────
  readonly totalModels = computed(() =>
    this.projects().flatMap(p => p.components).flatMap(c => c.features)
      .reduce((s, f) => s + f.models.length, 0),
  );

  constructor() {
    void this.refresh();
  }

  /** Reads the whole tree from the backend. */
  async refresh(): Promise<void> {
    try {
      const projects = await this.api.listProjects();
      const tree = await Promise.all(projects.map(async p => {
        const comps = await this.api.listComponents(p.id);
        const components = await Promise.all(comps.map(async c => {
          const feats = await this.api.listFeatures(c.id);
          const features = await Promise.all(feats.map(async f => ({
            id: f.id, name: f.name, description: f.description ?? '',
            models: (await this.api.listModels(f.id)).map(toExplorerModel),
            expanded: true,
          })));
          return { id: c.id, name: c.name, features, expanded: true };
        }));
        return { id: p.id, name: p.name, components, expanded: true };
      }));
      this.projects.set(tree);
      this.error.set(null);
    } catch (e) {
      this.fail(e);
    } finally {
      this.loaded.set(true);
    }
  }

  private fail(e: unknown): void {
    this.error.set(e instanceof ApiError ? e.message : 'The server could not be reached.');
  }

  /** Runs a backend change; on failure shows the error and rethrows nothing. */
  private async run<T>(change: () => Promise<T>): Promise<T | null> {
    try {
      const result = await change();
      this.error.set(null);
      return result;
    } catch (e) {
      this.fail(e);
      return null;
    }
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

  /** Keep the tree's cached name, status and counts in step with the editor. */
  syncModelStats(
    modelId: string,
    patch: {
      name?: string;
      status?: ExplorerModel['status'];
      states?: number;
      transitions?: number;
    },
  ): void {
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

  // ── Selection ──────────────────────────────────────────────────────────────
  // Lives here, not on a page, because the tree is rendered in the nav bar and
  // the Explorer page only shows the detail of whatever the tree selected.
  readonly selection = signal<ExplorerSelection | null>(null);

  readonly selectedProject = computed(() => {
    const s = this.selection();
    return s ? this.projects().find(p => p.id === s.projectId) ?? null : null;
  });

  readonly selectedComponent = computed(() => {
    const s = this.selection();
    if (!s || s.kind === 'project') return null;
    return this.selectedProject()?.components.find(c => c.id === s.componentId) ?? null;
  });

  readonly selectedFeature = computed(() => {
    const s = this.selection();
    if (!s || s.kind === 'project' || s.kind === 'component') return null;
    return this.selectedComponent()?.features.find(f => f.id === s.featureId) ?? null;
  });

  readonly selectedModel = computed(() => {
    const s = this.selection();
    if (s?.kind !== 'model') return null;
    return this.selectedFeature()?.models.find(m => m.id === s.modelId) ?? null;
  });

  select(sel: ExplorerSelection | null): void { this.selection.set(sel); }

  // ── "New item" dialog ──────────────────────────────────────────────────────
  // The request lives here so the Explorer page can open the dialog that the
  // tree — the component that is always mounted — renders.
  readonly newItemPrompt = signal<NewItemPrompt | null>(null);

  promptNew(prompt: NewItemPrompt): void { this.newItemPrompt.set(prompt); }

  closePrompt(): void { this.newItemPrompt.set(null); }

  /** True when `id` is the selected node, compared at its own level. */
  isSelected(id: string): boolean {
    const s = this.selection();
    if (!s) return false;
    switch (s.kind) {
      case 'project':   return s.projectId === id;
      case 'component': return s.componentId === id;
      case 'feature':   return s.featureId === id;
      case 'model':     return s.modelId === id;
    }
  }

  // ── Project mutations ──────────────────────────────────────────────────────
  async addProject(name: string): Promise<ExplorerProject | null> {
    const created = await this.run(() => this.api.createProject(name));
    if (!created) return null;
    const p: ExplorerProject = { id: created.id, name: created.name, components: [], expanded: true };
    this.projects.update(ps => [...ps, p]);
    return p;
  }

  async renameProject(id: string, name: string): Promise<void> {
    if (!await this.run(() => this.api.renameProject(id, name))) return;
    this.projects.update(ps => ps.map(p => p.id === id ? { ...p, name } : p));
  }

  async deleteProject(id: string): Promise<void> {
    if (!await this.run(() => this.api.deleteProject(id).then(() => true))) return;
    this.projects.update(ps => ps.filter(p => p.id !== id));
  }

  toggleProject(id: string): void {
    this.projects.update(ps => ps.map(p => p.id === id ? { ...p, expanded: !p.expanded } : p));
  }

  // ── Component mutations ────────────────────────────────────────────────────
  async addComponent(projectId: string, name: string): Promise<ExplorerComponent | null> {
    const created = await this.run(() => this.api.createComponent(projectId, name));
    if (!created) return null;
    const c: ExplorerComponent = { id: created.id, name: created.name, features: [], expanded: true };
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? { ...p, components: [...p.components, c] } : p,
    ));
    return c;
  }

  async renameComponent(projectId: string, componentId: string, name: string): Promise<void> {
    if (!await this.run(() => this.api.renameComponent(componentId, name))) return;
    this.projects.update(ps => ps.map(p =>
      p.id === projectId ? {
        ...p,
        components: p.components.map(c => c.id === componentId ? { ...c, name } : c),
      } : p,
    ));
  }

  async deleteComponent(projectId: string, componentId: string): Promise<void> {
    if (!await this.run(() => this.api.deleteComponent(componentId).then(() => true))) return;
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
  async addFeature(projectId: string, componentId: string, name: string): Promise<ExplorerFeature | null> {
    const created = await this.run(() => this.api.createFeature(componentId, name));
    if (!created) return null;
    const f: ExplorerFeature = {
      id: created.id, name: created.name, description: created.description ?? '',
      models: [], expanded: true,
    };
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

  async renameFeature(projectId: string, componentId: string, featureId: string, name: string): Promise<void> {
    if (!await this.run(() => this.api.renameFeature(featureId, name))) return;
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

  async deleteFeature(projectId: string, componentId: string, featureId: string): Promise<void> {
    if (!await this.run(() => this.api.deleteFeature(featureId).then(() => true))) return;
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
  async addModel(
    projectId: string, componentId: string, featureId: string, name: string,
  ): Promise<ExplorerModel | null> {
    const created = await this.run(() => this.api.createModel(featureId, { name }));
    if (!created) return null;
    const m = toExplorerModel(created);
    this.mapFeature(projectId, componentId, featureId,
      f => ({ ...f, models: [...f.models, m], expanded: true }));
    return m;
  }

  async renameModel(
    projectId: string, componentId: string, featureId: string, modelId: string, name: string,
  ): Promise<void> {
    if (!await this.run(() => this.api.renameModel(modelId, name))) return;
    this.mapFeature(projectId, componentId, featureId,
      f => ({ ...f, models: f.models.map(m => m.id === modelId ? { ...m, name } : m) }));
  }

  async deleteModel(
    projectId: string, componentId: string, featureId: string, modelId: string,
  ): Promise<void> {
    if (!await this.run(() => this.api.deleteModel(modelId).then(() => true))) return;
    this.removeModel(projectId, componentId, featureId, modelId);
  }

  /** Drops a model from the tree only; the caller has already deleted it. */
  removeModel(projectId: string, componentId: string, featureId: string, modelId: string): void {
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
}
