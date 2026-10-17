import { inject, Injectable } from '@angular/core';
import { Router } from '@angular/router';
import { TabSlug } from './threat-model-view';

/**
 * Moves between the tabs of the open workspace, keeping the project. A
 * selection or filter travels as a query parameter (`select`, `target`, …),
 * so the link is shareable too. Provided by the workspace page.
 */
@Injectable()
export class WorkspaceNav {
  private readonly router = inject(Router);
  projectId: string | null = null;

  go(tab: TabSlug, query: Record<string, string> = {}): void {
    if (!this.projectId) return;
    void this.router.navigate(['/threat-modeling', this.projectId, tab], { queryParams: query });
  }
}
