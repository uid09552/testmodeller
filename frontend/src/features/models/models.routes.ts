import { Routes } from '@angular/router';

export const modelRoutes: Routes = [
  // Models are browsed from the Explorer tree, so there is no model index.
  // `/models/new` and `/models/:modelId` remain as the Explorer's deep links.
  // (pages/model-list-page.* is kept on disk but no longer routed.)
  {
    path: '',
    redirectTo: '/explorer',
    pathMatch: 'full',
  },
  {
    path: 'new',
    loadComponent: () =>
      import('./pages/model-editor-page').then((m) => m.ModelEditorPageComponent),
    data: { breadcrumb: 'New Model' },
  },
  {
    path: ':modelId',
    loadComponent: () =>
      import('./pages/model-editor-page').then((m) => m.ModelEditorPageComponent),
    data: { breadcrumb: 'Model Editor' },
  },
];
