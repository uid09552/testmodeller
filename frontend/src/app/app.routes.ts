import { Routes } from '@angular/router';
import { ShellComponent } from '../core/layout/shell';

export const routes: Routes = [
  {
    path: '',
    component: ShellComponent,
    children: [
      {
        path: '',
        redirectTo: 'explorer',
        pathMatch: 'full',
      },
      {
        path: 'explorer',
        data: { breadcrumb: 'Explorer' },
        loadChildren: () =>
          import('../features/explorer/explorer.routes').then((m) => m.explorerRoutes),
      },
      {
        path: 'models',
        data: { breadcrumb: 'Models' },
        loadChildren: () =>
          import('../features/models/models.routes').then((m) => m.modelRoutes),
      },
      {
        path: 'test-cases',
        data: { breadcrumb: 'Test Cases' },
        loadChildren: () =>
          import('../features/test-cases/test-cases.routes').then((m) => m.testCaseRoutes),
      },
      {
        path: 'coverage',
        data: { breadcrumb: 'Coverage' },
        loadComponent: () =>
          import('../features/coverage/coverage-dashboard').then((m) => m.CoverageDashboardComponent),
      },
      {
        path: 'proposals',
        data: { breadcrumb: 'AI Proposals' },
        loadComponent: () =>
          import('../features/proposals/proposal-list').then((m) => m.ProposalListComponent),
      },
      {
        path: 'settings',
        data: { breadcrumb: 'Settings' },
        loadComponent: () =>
          import('../features/settings/pages/settings-page').then((m) => m.SettingsPageComponent),
      },
      {
        path: 'profile',
        data: { breadcrumb: 'Profile' },
        loadComponent: () =>
          import('../features/settings/pages/profile-page').then((m) => m.ProfilePageComponent),
      },
    ],
  },
];
