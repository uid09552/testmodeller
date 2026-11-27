import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { toApiError } from './api-error';
import { ApiKey, ApiKeyCreated, ApiKeyInput } from './api.types';

/** Base path of the API; the dev server proxies it (frontend/proxy.conf.json). */
const BASE = '/api/v1';

/**
 * The caller's own API keys for the MCP endpoint (tag `ApiKeys` in
 * openapi.yaml; 08-usermanagement.md → API keys).
 */
@Injectable({ providedIn: 'root' })
export class ApiKeysApi {
  private readonly http = inject(HttpClient);

  /** `GET /me/api-keys` */
  list(): Promise<ApiKey[]> {
    return this.call(() => firstValueFrom(this.http.get<ApiKey[]>(`${BASE}/me/api-keys`)));
  }

  /** `POST /me/api-keys` — the response is the only time the key is returned. */
  create(input: ApiKeyInput): Promise<ApiKeyCreated> {
    return this.call(() =>
      firstValueFrom(this.http.post<ApiKeyCreated>(`${BASE}/me/api-keys`, input)));
  }

  /** `DELETE /me/api-keys/{id}` */
  revoke(id: string): Promise<void> {
    return this.call(() =>
      firstValueFrom(this.http.delete<void>(`${BASE}/me/api-keys/${encodeURIComponent(id)}`)));
  }

  private async call<T>(fn: () => Promise<T>): Promise<T> {
    try {
      return await fn();
    } catch (e) {
      throw toApiError(e);
    }
  }
}
