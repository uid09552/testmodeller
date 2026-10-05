import { Routes } from '@angular/router';

export const modelRoutes: Routes = [
  // Models are opened from the Projects tree in the nav bar; there is no model
  // index or overview screen (ADR 0004).
  {
    path: '',
    redirectTo: '/explorer',
    pathMatch: 'full',
  },
  {
    path: ':modelId',
    loadComponent: () =>
      import('./pages/model-editor-page').then((m) => m.ModelEditorPageComponent),
    data: { breadcrumb: 'Model Editor' },
  },
];
