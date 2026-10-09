import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  ExplorerStore, ExplorerComponent, ExplorerFeature, ExplorerModel,
} from '../state/explorer.store';

/**
 * Detail view for whatever the nav-bar tree has selected.
 *
 * The tree itself lives in the nav bar (ADR 0004), and models open straight in
 * the editor, so this page covers projects, components and features only.
 */
@Component({
  selector: 'tm-explorer-page',
  templateUrl: './explorer-page.html',
  styleUrl:    './explorer-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExplorerPageComponent {
  readonly store  = inject(ExplorerStore);
  private readonly router = inject(Router);

  readonly selProject   = this.store.selectedProject;
  readonly selComponent = this.store.selectedComponent;
  readonly selFeature   = this.store.selectedFeature;

  /** Open a model in the editor, carrying the feature context with it. */
  openModel(model: ExplorerModel, _feature: ExplorerFeature): void {
    void this.router.navigate(['/models', model.id]);
  }

  selectComponent(component: ExplorerComponent): void {
    const p = this.selProject();
    if (p) this.store.select({ kind: 'component', projectId: p.id, componentId: component.id });
  }

  selectFeature(component: ExplorerComponent, feature: ExplorerFeature): void {
    const p = this.selProject();
    if (p) {
      this.store.select({
        kind: 'feature', projectId: p.id, componentId: component.id, featureId: feature.id,
      });
    }
  }
}
