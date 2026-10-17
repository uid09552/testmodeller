import { Routes } from '@angular/router';
import { WorkspacePageComponent } from './pages/workspace-page';

/**
 * `/threat-modeling` opens the first project; `/threat-modeling/:projectId/:tab`
 * names the project and the tab, so a link reopens both (06-ui.md).
 */
export const threatModelRoutes: Routes = [
  { path: '', component: WorkspacePageComponent },
  { path: ':projectId', component: WorkspacePageComponent },
  { path: ':projectId/:tab', component: WorkspacePageComponent },
];
