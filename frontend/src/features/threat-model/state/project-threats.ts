import { inject, Injectable, InjectionToken, Signal } from '@angular/core';
import { ApiError } from '../../../core/api/api-error';
import { ThreatModelApi } from '../../../core/api/threat-model-api';

/** A threat a feature test case can cover (ADR 0016). */
export interface ThreatOption {
  id: string;
  /** "THR-3 Account takeover". */
  label: string;
}

/**
 * The project whose threats a test case editor offers. The model editor page
 * provides it for its panels; elsewhere the host passes `projectId` itself.
 */
export const EDITOR_PROJECT_ID = new InjectionToken<Signal<string | null>>('EDITOR_PROJECT_ID');

/** Threats of a project's threat model, as options for the test case editor. */
@Injectable({ providedIn: 'root' })
export class ProjectThreats {
  private readonly api = inject(ThreatModelApi);
  private readonly cache = new Map<string, Promise<ThreatOption[]>>();

  /**
   * The project's threats, newest numbering order; empty when the project has
   * no threat model. Cached per project; `refresh` drops the cached list.
   */
  options(projectId: string, refresh = false): Promise<ThreatOption[]> {
    if (refresh) this.cache.delete(projectId);
    let p = this.cache.get(projectId);
    if (!p) {
      p = this.load(projectId);
      this.cache.set(projectId, p);
      // A failure is not cached, so the next editor tries again.
      p.catch(() => this.cache.delete(projectId));
    }
    return p;
  }

  private async load(projectId: string): Promise<ThreatOption[]> {
    let tmId: string;
    try {
      tmId = (await this.api.ofProject(projectId)).id;
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) return [];
      throw e;
    }
    const threats = await this.api.list(tmId, 'threats');
    // Dismissed threats stay listed: a test case may already cover one.
    return threats.map(t => ({
      id: t.id,
      label: `${t.displayId} ${t.title}${t.status === 'dismissed' ? ' (dismissed)' : ''}`,
    }));
  }
}
