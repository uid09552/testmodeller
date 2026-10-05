import { HttpErrorResponse } from '@angular/common/http';
import { Problem } from './api.types';

/** An API failure carrying whatever the server explained about it. */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly problem?: Problem,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Turns an `HttpErrorResponse` into an `ApiError` with a message worth showing.
 *
 * The backend answers with RFC 7807 problem details, so prefer `detail`, then
 * `title`; a status of 0 means the request never reached the server.
 */
export function toApiError(e: unknown): ApiError {
  if (e instanceof ApiError) return e;
  if (!(e instanceof HttpErrorResponse)) {
    return new ApiError(e instanceof Error ? e.message : 'Unexpected error', 0);
  }
  if (e.status === 0) {
    return new ApiError(
      'Cannot reach the TestModeller API. Is the backend running?',
      0,
    );
  }
  const problem: Problem | undefined =
    e.error && typeof e.error === 'object' ? (e.error as Problem) : undefined;
  const fields = problem?.errors?.map(f => `${f.field}: ${f.message}`).join('; ');
  const message = [problem?.detail ?? problem?.title ?? e.message, fields]
    .filter(Boolean)
    .join(' — ');
  return new ApiError(message, e.status, problem);
}
