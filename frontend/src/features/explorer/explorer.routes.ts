import { Routes } from '@angular/router';

export const explorerRoutes: Routes = [
  {
    path: '',
    loadComponent: () => import('./pages/explorer-page').then((m) => m.ExplorerPageComponent),
  },
];
