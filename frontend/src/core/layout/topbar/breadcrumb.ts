import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRouteSnapshot, NavigationEnd, Router, RouterLink } from '@angular/router';
import { IxBreadcrumb, IxBreadcrumbItem } from '@siemens/ix-angular/standalone';
import { filter, map, startWith } from 'rxjs';
import { ExplorerStore } from '../../../features/explorer/state/explorer.store';

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

/** The deepest `modelId` route parameter in the chain, if any. */
export function modelIdOf(root: ActivatedRouteSnapshot | null | undefined): string | null {
  let id: string | null = null;
  for (let r = root ?? null; r; r = r.firstChild) id = r.params?.['modelId'] ?? id;
  return id;
}

/** Where a model sits in the tree, as the explorer store reports it. */
export interface ModelPlace {
  projectName: string;
  componentName: string;
  featureName: string;
  model: { id: string; name: string };
}

/**
 * Project > Component > Feature > Model for an open model (DESIGN.md, Layout).
 * The tree has no page per node, so the levels above the model lead to the
 * Explorer, where the tree is.
 */
export function modelCrumbs(place: ModelPlace): Crumb[] {
  return [
    { label: 'Projects', route: '/explorer' },
    { label: place.projectName, route: '/explorer' },
    { label: place.componentName, route: '/explorer' },
    { label: place.featureName, route: '/explorer' },
    { label: place.model.name },
  ];
}

/**
 * Reads `data.breadcrumb` from the active route chain; on a model, shows where
 * it sits in the project tree instead.
 */
@Component({
  selector: 'tm-breadcrumb',
  imports: [RouterLink, IxBreadcrumb, IxBreadcrumbItem],
  template: `
    <ix-breadcrumb aria-label="Breadcrumb">
      @for (crumb of crumbs(); track $index; let last = $last, i = $index) {
        @if (!last && crumb.route) {
          <ix-breadcrumb-item [breadcrumbKey]="i + ':' + crumb.route" [label]="crumb.label" [routerLink]="crumb.route" />
        } @else {
          <ix-breadcrumb-item [breadcrumbKey]="i + ':' + crumb.label" [label]="crumb.label" aria-current="page" />
        }
      }
    </ix-breadcrumb>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BreadcrumbComponent {
  private readonly router = inject(Router);
  private readonly explorer = inject(ExplorerStore);

  private readonly route = toSignal(
    this.router.events.pipe(
      filter((e) => e instanceof NavigationEnd),
      startWith(null),
      map(() => {
        const root = this.router.routerState.snapshot.root;
        return { crumbs: buildCrumbs(root), modelId: modelIdOf(root) };
      }),
    ),
    { initialValue: { crumbs: [] as Crumb[], modelId: null as string | null } },
  );

  readonly crumbs = computed(() => {
    const { crumbs, modelId } = this.route();
    const place = modelId ? this.explorer.modelPaths().find(p => p.model.id === modelId) : undefined;
    return place ? modelCrumbs(place) : crumbs;
  });
}
