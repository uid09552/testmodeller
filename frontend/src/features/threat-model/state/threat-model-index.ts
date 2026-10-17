import { effect, inject, Injectable, signal, untracked } from '@angular/core';
import { ApiError } from '../../../core/api/api-error';
import { ThreatModelApi } from '../../../core/api/threat-model-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';

/** Which projects have a threat model, for the project tree. */
export interface ThreatModelRef { id: string; name: string }

/**
 * Knows, per project, whether it has a threat model and what it is called, so
 * the project tree can list it under the project. The workspace keeps it
 * current when a threat model is created or loaded.
 */
@Injectable({ providedIn: 'root' })
export class ThreatModelIndex {
  private readonly api = inject(ThreatModelApi);
  private readonly explorer = inject(ExplorerStore);

  /** Project id → its threat model, or null when it has none; absent while unknown. */
  readonly byProject = signal<Record<string, ThreatModelRef | null>>({});

  constructor() {
    effect(() => {
      const ids = this.explorer.projects().map(p => p.id);
      untracked(() => void this.load(ids.filter(id => !(id in this.byProject()))));
    });
  }

  of(projectId: string): ThreatModelRef | null {
    return this.byProject()[projectId] ?? null;
  }

  set(projectId: string, ref: ThreatModelRef | null): void {
    this.byProject.update(m => ({ ...m, [projectId]: ref }));
  }

  private async load(ids: string[]): Promise<void> {
    await Promise.all(ids.map(async id => {
      try {
        const tm = await this.api.ofProject(id);
        this.set(id, { id: tm.id, name: tm.name });
      } catch (e) {
        // A project without a threat model answers 404; anything else is retried on the next load.
        if (e instanceof ApiError && e.status === 404) this.set(id, null);
      }
    }));
  }

  /** Creates a threat model for a project and records it. */
  async create(projectId: string, name: string): Promise<ThreatModelRef> {
    const tm = await this.api.create(projectId, { name });
    const ref = { id: tm.id, name: tm.name };
    this.set(projectId, ref);
    return ref;
  }

  /** Deletes a project's threat model and everything in it. */
  async remove(projectId: string): Promise<void> {
    const ref = this.of(projectId);
    if (!ref) return;
    await this.api.deleteModel(ref.id);
    this.set(projectId, null);
  }
}
