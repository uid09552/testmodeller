import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRouteSnapshot, NavigationEnd, Router, RouterLink } from '@angular/router';
import { filter, map, startWith } from 'rxjs';

export interface Crumb {
  label: string;
  route?: string;
}

/**
 * Crumbs for the route chain, from `data.breadcrumb`.
 *
 * Walks the router's snapshot tree rather than `ActivatedRoute` objects:
 * `ActivatedRoute.snapshot` is unset on a route that has not finished
 * activating, and reading `.data` off it threw on navigation. Each crumb links
 * to the absolute path up to its level, so a crumb deep in the chain does not
 * resolve against the top bar's own route.
 */
export function buildCrumbs(root: ActivatedRouteSnapshot | null | undefined): Crumb[] {
  const crumbs: Crumb[] = [];
  const segments: string[] = [];
  let current = root ?? null;
  while (current) {
    segments.push(...current.url.map(u => u.path));
    const label = current.data?.['breadcrumb'];
    // A route without its own path (a lazy-loaded child at '') repeats its
    // parent's label; one crumb per label is enough.
    if (typeof label === 'string' && crumbs.at(-1)?.label !== label) {
      crumbs.push({ label, route: '/' + segments.filter(Boolean).join('/') });
    }
    current = current.firstChild;
  }
  return crumbs;
}

/** Reads `data.breadcrumb` from the active route chain. */
@Component({
  selector: 'tm-breadcrumb',
  imports: [RouterLink],
  template: `
    <nav class="breadcrumb" aria-label="Breadcrumb">
      <ol>
        @for (crumb of crumbs(); track crumb.label; let last = $last) {
          <li [class.breadcrumb__current]="last" [attr.aria-current]="last ? 'page' : null">
            @if (!last && crumb.route) {
              <a [routerLink]="crumb.route">{{ crumb.label }}</a>
            } @else {
              <span>{{ crumb.label }}</span>
            }
            @if (!last) { <span class="breadcrumb__sep" aria-hidden="true">/</span> }
          </li>
        }
      </ol>
    </nav>
  `,
  styleUrl: './breadcrumb.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BreadcrumbComponent {
  private readonly router = inject(Router);

  readonly crumbs = toSignal(
    this.router.events.pipe(
      filter((e) => e instanceof NavigationEnd),
      startWith(null),
      map(() => buildCrumbs(this.router.routerState.snapshot.root)),
    ),
    { initialValue: [] as Crumb[] },
  );
}
