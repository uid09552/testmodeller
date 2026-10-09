import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { toApiError } from './api-error';
import { TestResult } from './api.types';

const BASE = '/api/v1';

/** Gap types of the `gap` query parameter. */
export type TraceGap = 'untraced' | 'unimplemented' | 'no-elements';

/** A state or transition a test case is assigned to; exactly one id is set. */
export interface TraceElement {
  modelId: string;
  stateId?: string;
  transitionId?: string;
}

export interface TraceTestCase {
  id: string;
  name: string;
  featureId: string;
  componentId: string;
  implementationUrl?: string;
  /** Latest imported result. */
  lastResult?: TestResult;
  elements: TraceElement[];
}

/** A backlog item (a requirement), keyed by its normalised URL. */
export interface TraceItem {
  backlogUrl: string;
  testCases: TraceTestCase[];
}

export interface TraceSummary {
  backlogItems: number;
  testCases: number;
  untraced: number;
  unimplemented: number;
  noElements: number;
}

export interface Traceability {
  items: TraceItem[];
  untraced: TraceTestCase[];
  summary: TraceSummary;
}

export interface TraceQuery {
  componentId?: string;
  featureId?: string;
  gap?: TraceGap;
}

/** `GET /projects/{projectId}/traceability` (tag `Traceability` in openapi.yaml). */
@Injectable({ providedIn: 'root' })
export class TraceabilityApi {
  private readonly http = inject(HttpClient);

  async get(projectId: string, query: TraceQuery = {}): Promise<Traceability> {
    const params: Record<string, string> = {};
    if (query.componentId) params['componentId'] = query.componentId;
    if (query.featureId) params['featureId'] = query.featureId;
    if (query.gap) params['gap'] = query.gap;
    try {
      return await firstValueFrom(
        this.http.get<Traceability>(`${BASE}/projects/${projectId}/traceability`, { params }));
    } catch (e) {
      throw toApiError(e);
    }
  }
}
