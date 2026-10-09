import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { toApiError } from './api-error';
import { TestResult } from './api.types';

const BASE = '/api/v1';

export type ResultFormat = 'junit' | 'cucumber';

/** `ImportReport` in openapi.yaml. */
export interface ImportReport {
  dryRun: boolean;
  total: number;
  matched: number;
  recorded: number;
  duplicates: number;
  unmatched: string[];
  ambiguous: string[];
}

/** Largest file the backend accepts. */
export const MAX_RESULT_FILE_BYTES = 10 * 1024 * 1024;

/** Import and history of test results (tag `TestResults` in openapi.yaml). */
@Injectable({ providedIn: 'root' })
export class TestResultsApi {
  private readonly http = inject(HttpClient);

  /** `POST /projects/{id}/test-results`: the file is the body, its name travels in Content-Disposition. */
  async import(projectId: string, format: ResultFormat, file: File, dryRun: boolean): Promise<ImportReport> {
    const contentType = format === 'junit' ? 'application/xml' : 'application/json';
    const name = file.name.replace(/["\\\r\n]/g, '');
    return this.call(() => firstValueFrom(this.http.post<ImportReport>(
      `${BASE}/projects/${projectId}/test-results`, file, {
        params: { format, dryRun: String(dryRun) },
        headers: { 'Content-Type': contentType, 'Content-Disposition': `attachment; filename="${name}"` },
      })));
  }

  async history(testCaseId: string, limit = 50): Promise<TestResult[]> {
    return this.call(() => firstValueFrom(this.http.get<TestResult[]>(
      `${BASE}/test-cases/${testCaseId}/results`, { params: { limit: String(limit) } })));
  }

  private async call<T>(fn: () => Promise<T>): Promise<T> {
    try {
      return await fn();
    } catch (e) {
      throw toApiError(e);
    }
  }
}

/** The format a file name suggests, if any. */
export function formatFromName(name: string): ResultFormat | null {
  const n = name.toLowerCase();
  if (n.endsWith('.xml')) return 'junit';
  if (n.endsWith('.json')) return 'cucumber';
  return null;
}
