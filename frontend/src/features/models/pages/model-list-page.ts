import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

interface ModelRow {
  id: string;
  name: string;
  feature: string;
  component: string;
  states: number;
  transitions: number;
  status: 'draft' | 'review' | 'approved';
  updated: string;
}

@Component({
  selector: 'tm-model-list-page',
  imports: [RouterLink],
  templateUrl: './model-list-page.html',
  styleUrl: './model-list-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ModelListPageComponent {
  private readonly router = inject(Router);
  readonly search = signal('');
  readonly statusFilter = signal<'all' | 'draft' | 'review' | 'approved'>('all');

  // Placeholder rows until GET /models is wired up.
  readonly models = signal<ModelRow[]>([
    { id: 'm1', name: 'Login Flow',        feature: 'Login',        component: 'Auth Flow', states: 4, transitions: 5, status: 'approved', updated: '2 hours ago' },
    { id: 'm2', name: 'Registration Flow', feature: 'Registration', component: 'Auth Flow', states: 6, transitions: 8, status: 'review',   updated: 'yesterday' },
    { id: 'm3', name: 'Reset Password',    feature: 'Password Reset', component: 'Auth Flow', states: 3, transitions: 3, status: 'draft',  updated: '3 days ago' },
    { id: 'm4', name: 'Cart Checkout',     feature: 'Cart',         component: 'Checkout',  states: 7, transitions: 11, status: 'draft',   updated: 'last week' },
  ]);

  readonly filtered = computed(() => {
    const q = this.search().toLowerCase().trim();
    const st = this.statusFilter();
    return this.models().filter(m =>
      (st === 'all' || m.status === st) &&
      (!q || m.name.toLowerCase().includes(q) || m.feature.toLowerCase().includes(q)),
    );
  });

  setStatus(s: 'all' | 'draft' | 'review' | 'approved'): void { this.statusFilter.set(s); }

  /** Double-click a card to open it in the editor. */
  open(m: ModelRow): void {
    this.router.navigate(['/models', m.id], {
      queryParams: { name: m.name, 'feature-name': m.feature },
    });
  }

  deleteModel(id: string): void {
    this.models.update(ms => ms.filter(m => m.id !== id));
  }
}
