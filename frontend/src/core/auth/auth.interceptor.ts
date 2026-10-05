import { HttpErrorResponse, HttpInterceptorFn } from '@angular/common/http';
import { inject } from '@angular/core';
import { catchError, throwError } from 'rxjs';
import { SessionService } from './session';

/**
 * Turns an expired gateway session into a fresh handshake.
 *
 * The SPA adds no `Authorization` header — the gateway attaches the token — so
 * this interceptor only reacts: a 401 from the API means the session is gone,
 * and the only way back is through the gateway, i.e. a reload. A 403 means the
 * session is fine but the role is not, which is the caller's problem to report.
 */
export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const session = inject(SessionService);
  return next(req).pipe(
    catchError((e: unknown) => {
      if (e instanceof HttpErrorResponse && e.status === 401) {
        session.reauthenticate();
      }
      return throwError(() => e);
    }),
  );
};
