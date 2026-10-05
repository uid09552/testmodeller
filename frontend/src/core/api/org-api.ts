import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { toApiError } from './api-error';
import {
  ModelInput, StateInput, TestCase, TestCaseInput, TransitionInput,
} from './api.types';

const BASE = '/api/v1';

/** Audit fields every stored entity carries. */
export interface Audit {
  id: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface Project extends Audit { name: string; description?: string }
export interface Component extends Audit { projectId: string; name: string }
export interface Feature extends Audit {
  componentId: string;
  name: string;
  description?: string;
  scenarioDescription?: string;
}
export interface ModelSummary extends Audit {
  featureId: string;
  name: string;
  status?: string;
  stateCount: number;
  transitionCount: number;
}
export interface Model extends ModelSummary {
  description?: string;
  status?: string;
  states: StateInput[];
  transitions: TransitionInput[];
}

/**
 * Projects, components, features and models (tags `Projects`, `Components`,
 * `Features` and `Models` in openapi.yaml).
 *
 * Used by `ExplorerStore` for the tree and by
 * `ModelPersistenceService` to store the model itself.
 */
@Injectable({ providedIn: 'root' })
export class OrgApi {
  private readonly http = inject(HttpClient);

  // ── Reading the tree ──────────────────────────────────────────────────────
  async listProjects(): Promise<Project[]> {
    return this.page<Project>(`${BASE}/projects`);
  }

  async listComponents(projectId: string): Promise<Component[]> {
    return this.page<Component>(`${BASE}/projects/${projectId}/components`);
  }

  async listFeatures(componentId: string): Promise<Feature[]> {
    return this.page<Feature>(`${BASE}/components/${componentId}/features`);
  }

  async listModels(featureId: string): Promise<ModelSummary[]> {
    return this.call(() => firstValueFrom(
      this.http.get<ModelSummary[]>(`${BASE}/features/${featureId}/models`)));
  }

  // ── Renaming and deleting ─────────────────────────────────────────────────
  async renameProject(id: string, name: string): Promise<Project> {
    return this.call(async () => {
      const cur = await firstValueFrom(this.http.get<Project>(`${BASE}/projects/${id}`));
      return firstValueFrom(this.http.patch<Project>(
        `${BASE}/projects/${id}`, { name, description: cur.description },
        { headers: { 'If-Match': String(cur.version) } }));
    });
  }

  async renameComponent(id: string, name: string): Promise<Component> {
    return this.call(async () => {
      const cur = await firstValueFrom(this.http.get<Component>(`${BASE}/components/${id}`));
      return firstValueFrom(this.http.patch<Component>(
        `${BASE}/components/${id}`, { name },
        { headers: { 'If-Match': String(cur.version) } }));
    });
  }

  async renameFeature(id: string, name: string): Promise<Feature> {
    const cur = await this.getFeature(id);
    return this.updateFeature(id, cur.version, { name });
  }

  async renameModel(id: string, name: string): Promise<ModelSummary> {
    return this.call(async () => {
      const cur = await firstValueFrom(this.http.get<Model>(`${BASE}/models/${id}`));
      return firstValueFrom(this.http.patch<ModelSummary>(
        `${BASE}/models/${id}`, { name },
        { headers: { 'If-Match': String(cur.version) } }));
    });
  }

  async deleteProject(id: string): Promise<void> {
    await this.call(() => firstValueFrom(this.http.delete<void>(`${BASE}/projects/${id}`)));
  }

  async deleteComponent(id: string): Promise<void> {
    await this.call(() => firstValueFrom(
      this.http.delete<void>(`${BASE}/components/${id}`, { params: { cascade: true } })));
  }

  async deleteFeature(id: string): Promise<void> {
    await this.call(() => firstValueFrom(
      this.http.delete<void>(`${BASE}/features/${id}`, { params: { cascade: true } })));
  }

  async getTestCase(id: string): Promise<TestCase> {
    return this.call(() =>
      firstValueFrom(this.http.get<TestCase>(`${BASE}/test-cases/${id}`)));
  }

  async createProject(name: string): Promise<Project> {
    return this.call(() => firstValueFrom(
      this.http.post<Project>(`${BASE}/projects`, { name })));
  }

  async createComponent(projectId: string, name: string): Promise<Component> {
    return this.call(() => firstValueFrom(
      this.http.post<Component>(`${BASE}/projects/${projectId}/components`, { name })));
  }

  async createFeature(
    componentId: string, name: string, scenarioDescription?: string,
  ): Promise<Feature> {
    return this.call(() => firstValueFrom(
      this.http.post<Feature>(`${BASE}/components/${componentId}/features`, {
        name, scenarioDescription,
      })));
  }

  async updateFeature(
    featureId: string, version: number, patch: Partial<Feature>,
  ): Promise<Feature> {
    return this.call(() => firstValueFrom(
      this.http.patch<Feature>(`${BASE}/features/${featureId}`, patch, {
        headers: { 'If-Match': String(version) },
      })));
  }

  async getFeature(featureId: string): Promise<Feature> {
    return this.call(() =>
      firstValueFrom(this.http.get<Feature>(`${BASE}/features/${featureId}`)));
  }

  async createModel(featureId: string, input: ModelInput): Promise<Model> {
    return this.call(() => firstValueFrom(
      this.http.post<Model>(`${BASE}/features/${featureId}/models`, input)));
  }

  async getModel(modelId: string): Promise<Model> {
    return this.call(() =>
      firstValueFrom(this.http.get<Model>(`${BASE}/models/${modelId}`)));
  }

  /** Atomic graph save; `If-Match` makes a concurrent edit a 412, not a clobber. */
  async replaceModel(modelId: string, version: number, input: ModelInput): Promise<Model> {
    return this.call(() => firstValueFrom(
      this.http.put<Model>(`${BASE}/models/${modelId}`, input, {
        headers: { 'If-Match': String(version) },
      })));
  }

  async deleteModel(modelId: string): Promise<void> {
    await this.call(() => firstValueFrom(
      this.http.delete<void>(`${BASE}/models/${modelId}`)));
  }

  // ── Test cases ────────────────────────────────────────────────────────────
  /** `GET /models/{id}/test-cases` — every test case assigned in the model. */
  async modelTestCases(modelId: string): Promise<TestCase[]> {
    return this.call(() => firstValueFrom(
      this.http.get<TestCase[]>(`${BASE}/models/${modelId}/test-cases`)));
  }

  async createTestCase(featureId: string, input: TestCaseInput): Promise<TestCase> {
    return this.call(() => firstValueFrom(
      this.http.post<TestCase>(`${BASE}/features/${featureId}/test-cases`, input)));
  }

  async replaceTestCase(id: string, version: number, input: TestCaseInput): Promise<TestCase> {
    return this.call(() => firstValueFrom(
      this.http.put<TestCase>(`${BASE}/test-cases/${id}`, input, {
        headers: { 'If-Match': String(version) },
      })));
  }

  async deleteTestCase(id: string): Promise<void> {
    await this.call(() => firstValueFrom(
      this.http.delete<void>(`${BASE}/test-cases/${id}`)));
  }

  /** Every item of a cursor-paged list. */
  private async page<T>(url: string): Promise<T[]> {
    const items: T[] = [];
    let cursor: string | null | undefined;
    do {
      const params: Record<string, string> = { limit: '200' };
      if (cursor) params['cursor'] = cursor;
      const res = await this.call(() => firstValueFrom(
        this.http.get<{ items: T[]; nextCursor?: string | null }>(url, { params })));
      items.push(...res.items);
      cursor = res.nextCursor;
    } while (cursor);
    return items;
  }

  private async call<T>(fn: () => Promise<T>): Promise<T> {
    try {
      return await fn();
    } catch (e) {
      throw toApiError(e);
    }
  }
}
