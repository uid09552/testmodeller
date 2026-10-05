import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { toApiError } from './api-error';
import {
  AiSettings, Job, Proposal, ProposalRequest,
} from './api.types';

/** Base path of the API; the dev server proxies it (frontend/proxy.conf.json). */
const BASE = '/api/v1';

/** How long to keep polling a proposal job before giving up. */
const JOB_TIMEOUT_MS = 180_000;
/** Delay between job polls. The contract says AI endpoints may be slow. */
const JOB_POLL_MS = 1_200;

/**
 * The AI endpoints of the contract (tag `AI` in openapi.yaml).
 *
 * Proposals run asynchronously: `request` returns a job, `awaitJob` polls it,
 * and the resulting proposals are fetched one by one. See ADR 0003 for why the
 * chat panel is built on these rather than on a chat endpoint.
 */
@Injectable({ providedIn: 'root' })
export class AiApi {
  private readonly http = inject(HttpClient);

  /** `POST /ai/proposals` — starts a job (202). */
  async requestProposals(req: ProposalRequest): Promise<Job> {
    return this.call(() =>
      firstValueFrom(this.http.post<Job>(`${BASE}/ai/proposals`, req)));
  }

  /** `GET /jobs/{id}` */
  async getJob(id: string): Promise<Job> {
    return this.call(() => firstValueFrom(this.http.get<Job>(`${BASE}/jobs/${id}`)));
  }

  /** `GET /proposals/{id}` */
  async getProposal(id: string): Promise<Proposal> {
    return this.call(() =>
      firstValueFrom(this.http.get<Proposal>(`${BASE}/proposals/${id}`)));
  }

  /**
   * `POST /proposals/{id}/accept` — creates the entity with `origin = ai`.
   * `payload` overrides the proposal's own payload when the user edited it.
   */
  async acceptProposal(id: string, payload?: unknown): Promise<Proposal> {
    return this.call(() => firstValueFrom(
      this.http.post<Proposal>(`${BASE}/proposals/${id}/accept`,
        payload === undefined ? {} : { payload })));
  }

  /** `POST /proposals/{id}/reject` */
  async rejectProposal(id: string, reason?: string): Promise<Proposal> {
    return this.call(() => firstValueFrom(
      this.http.post<Proposal>(`${BASE}/proposals/${id}/reject`,
        reason ? { reason } : {})));
  }

  /** `GET /settings/ai` — used to tell the user when no provider is set up. */
  async getSettings(): Promise<AiSettings> {
    return this.call(() =>
      firstValueFrom(this.http.get<AiSettings>(`${BASE}/settings/ai`)));
  }


  /**
   * Polls a job until it succeeds or fails.
   *
   * `signal` lets a caller stop waiting; the job itself keeps running on the
   * server, which is why the panel can show it again later.
   */
  async awaitJob(id: string, signal?: AbortSignal): Promise<Job> {
    const deadline = Date.now() + JOB_TIMEOUT_MS;
    for (;;) {
      const job = await this.getJob(id);
      if (job.status === 'succeeded' || job.status === 'failed') return job;
      if (signal?.aborted) throw new DOMException('Aborted', 'AbortError');
      if (Date.now() > deadline) {
        return {
          id,
          status: 'failed',
          error: { detail: 'The AI job did not finish in time.' },
        };
      }
      await sleep(JOB_POLL_MS, signal);
    }
  }

  private async call<T>(fn: () => Promise<T>): Promise<T> {
    try {
      return await fn();
    } catch (e) {
      throw toApiError(e);
    }
  }
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      signal?.removeEventListener('abort', onAbort);
      resolve();
    }, ms);
    const onAbort = () => {
      clearTimeout(timer);
      reject(new DOMException('Aborted', 'AbortError'));
    };
    signal?.addEventListener('abort', onAbort, { once: true });
  });
}
