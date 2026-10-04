import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, NavigationEnd, Router, RouterLink } from '@angular/router';
import { filter, map, startWith } from 'rxjs';

export interface Crumb {
  label: string;
  route?: string;
}

function buildCrumbs(route: ActivatedRoute): Crumb[] {
  const crumbs: Crumb[] = [];
  let current: ActivatedRoute | null = route.root;
  while (current) {
    const data = current.snapshot.data;
    if (data['breadcrumb']) {
      crumbs.push({ label: data['breadcrumb'] as string, route: current.snapshot.url.join('/') });
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
  private readonly route = inject(ActivatedRoute);

  readonly crumbs = toSignal(
    this.router.events.pipe(
      filter((e) => e instanceof NavigationEnd),
      startWith(null),
      map(() => buildCrumbs(this.route)),
    ),
    { initialValue: [] as Crumb[] },
  );
}
