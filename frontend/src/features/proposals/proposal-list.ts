import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';

type ProposalKind = 'model' | 'states-and-transitions' | 'feature-description' | 'test-cases';
type ProposalStatus = 'pending' | 'accepted' | 'rejected';

interface Proposal {
  id: string;
  kind: ProposalKind;
  target: string;
  summary: string;
  /** Human-readable rendering of what would be added if accepted. */
  items: string[];
  status: ProposalStatus;
  createdAt: string;
  expanded: boolean;
}

@Component({
  selector: 'tm-proposal-list',
  templateUrl: './proposal-list.html',
  styleUrl: './proposal-list.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProposalListComponent {
  readonly filter = signal<'all' | ProposalStatus>('pending');

  // Placeholder data until GET /ai/proposals is wired up.
  readonly proposals = signal<Proposal[]>([
    {
      id: 'p1', kind: 'states-and-transitions', target: 'Login Flow',
      summary: 'Add error-handling states for failed authentication',
      status: 'pending', createdAt: '10 minutes ago', expanded: false,
      items: [
        'State "Auth Failed" (regular)',
        'State "Account Locked" (final)',
        'Transition Login → Auth Failed  [attempts < 3]',
        'Transition Auth Failed → Login  [retry]',
        'Transition Auth Failed → Account Locked  [attempts >= 3]',
      ],
    },
    {
      id: 'p2', kind: 'test-cases', target: 'Registration Flow',
      summary: 'Generate 4 boundary test cases for the registration form',
      status: 'pending', createdAt: '1 hour ago', expanded: false,
      items: [
        'Register with minimum-length password',
        'Register with maximum-length email',
        'Register with an already-used email',
        'Register with mismatched password confirmation',
      ],
    },
    {
      id: 'p3', kind: 'feature-description', target: 'Password Reset',
      summary: 'Draft a scenario description from the current model',
      status: 'pending', createdAt: '3 hours ago', expanded: false,
      items: [
        'As a user who has forgotten their password, I want to request a reset link by email so that I can regain access to my account without contacting support.',
      ],
    },
    {
      id: 'p4', kind: 'model', target: 'Checkout › Cart',
      summary: 'Propose an initial model for the cart feature',
      status: 'accepted', createdAt: 'yesterday', expanded: false,
      items: ['7 states, 11 transitions'],
    },
    {
      id: 'p5', kind: 'test-cases', target: 'Login Flow',
      summary: 'Generate exhaustive all-paths suite (38 cases)',
      status: 'rejected', createdAt: '2 days ago', expanded: false,
      items: ['38 test cases — rejected as too broad for this iteration'],
    },
  ]);

  readonly filtered = computed(() => {
    const f = this.filter();
    return this.proposals().filter(p => f === 'all' || p.status === f);
  });

  readonly pendingCount = computed(() => this.proposals().filter(p => p.status === 'pending').length);

  setFilter(f: 'all' | ProposalStatus): void { this.filter.set(f); }

  toggle(id: string): void {
    this.proposals.update(ps => ps.map(p => p.id === id ? { ...p, expanded: !p.expanded } : p));
  }

  /**
   * Explicit user approval is the only path that marks a proposal accepted —
   * nothing here is applied automatically (AGENTS.md rule 8).
   */
  accept(id: string): void {
    this.proposals.update(ps => ps.map(p => p.id === id ? { ...p, status: 'accepted' as const } : p));
  }

  reject(id: string): void {
    this.proposals.update(ps => ps.map(p => p.id === id ? { ...p, status: 'rejected' as const } : p));
  }

  kindLabel(k: ProposalKind): string {
    return {
      'model': 'Model',
      'states-and-transitions': 'States & Transitions',
      'feature-description': 'Feature Description',
      'test-cases': 'Test Cases',
    }[k];
  }
}
