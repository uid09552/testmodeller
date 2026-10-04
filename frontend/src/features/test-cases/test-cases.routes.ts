import { Routes } from '@angular/router';

export const testCaseRoutes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./pages/test-case-list-page').then((m) => m.TestCaseListPageComponent),
  },
];
