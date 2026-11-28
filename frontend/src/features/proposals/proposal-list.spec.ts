import { CUSTOM_ELEMENTS_SCHEMA, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  IxButton, IxCard, IxCardContent, IxEmptyState, IxMessageBar, IxPill, IxSelect, IxSelectItem,
  IxSpinner, ToastService,
} from '@siemens/ix-angular/standalone';
import { AiApi } from '../../core/api/ai-api';
import { ApiError } from '../../core/api/api-error';
import { Proposal } from '../../core/api/api.types';
import { ExplorerStore } from '../explorer/state/explorer.store';
import { ProposalListComponent } from './proposal-list';
import { details, summary } from './proposal-view';

const INERT = {
  remove: { imports: [IxButton, IxCard, IxCardContent, IxEmptyState, IxMessageBar, IxPill, IxSelect, IxSelectItem, IxSpinner] },
  add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
};

const proposal = (id: string, over: Partial<Proposal> = {}): Proposal => ({
  id, projectId: 'p1', featureId: 'f1', kind: 'test-cases', status: 'pending', createdAt: '2026-11-28T10:00:00Z',
  payload: { name: `Case ${id}`, steps: [{ action: 'log in', expected: 'home' }] },
  rationale: 'Covers the happy path.',
  ...over,
});

const PROJECTS = [{
  id: 'p1', name: 'Shop', expanded: true,
  components: [{ id: 'c1', name: 'Checkout', expanded: true,
    features: [{ id: 'f1', name: 'Cart', description: '', models: [], expanded: true }] }],
}];

describe('ProposalListComponent', () => {
  async function render(list: Proposal[]) {
    const api = {
      listProposals: vi.fn(async () => list),
      acceptProposal: vi.fn(async (id: string) => ({ ...list.find(p => p.id === id)!, status: 'accepted' })),
      rejectProposal: vi.fn(async (id: string) => ({ ...list.find(p => p.id === id)!, status: 'rejected' })),
    };
    const toasts = { show: vi.fn(async () => undefined) };
    TestBed.configureTestingModule({
      imports: [ProposalListComponent],
      providers: [
        { provide: AiApi, useValue: api },
        { provide: ToastService, useValue: toasts },
        { provide: ExplorerStore, useValue: { projects: signal(PROJECTS), modelPaths: signal([]) } },
      ],
    });
    TestBed.overrideComponent(ProposalListComponent, INERT);
    const f = TestBed.createComponent(ProposalListComponent);
    f.detectChanges();
    await f.whenStable();
    for (let i = 0; i < 3; i++) await new Promise(r => setTimeout(r));
    f.detectChanges();
    return { f, api, toasts, root: f.nativeElement as HTMLElement };
  }

  it("lists the project's pending proposals as proposals, not as accepted data", async () => {
    const { f, api, root } = await render([proposal('a'), proposal('b', { status: 'accepted' })]);
    expect(api.listProposals).toHaveBeenCalledWith('p1');
    expect(f.componentInstance.shown().map(p => p.id)).toEqual(['a']);
    const card = root.querySelector('ix-card')!;
    expect(card.querySelector('ix-pill')!.textContent).toContain('Proposal');
    expect(card.textContent).toContain('Test case "Case a"');
    expect(card.textContent).toContain('Covers the happy path.');
    expect(card.textContent).toContain('Checkout › Cart');
    const buttons = [...card.querySelectorAll('ix-button')].map(b => [b.textContent!.trim(), b.getAttribute('variant')]);
    expect(buttons).toEqual([['Reject', 'secondary'], ['Accept', 'primary']]);
  });

  it('accepts only on an explicit decision, and shows the result', async () => {
    const { f, api, toasts } = await render([proposal('a')]);
    await f.componentInstance.accept(f.componentInstance.shown()[0]);
    expect(api.acceptProposal).toHaveBeenCalledWith('a');
    expect(f.componentInstance.proposals()[0].status).toBe('accepted');
    expect(toasts.show).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('keeps a proposal pending when the decision fails', async () => {
    const { f, api, toasts } = await render([proposal('a')]);
    api.rejectProposal.mockRejectedValueOnce(new ApiError('conflict', 409));
    await f.componentInstance.reject(f.componentInstance.proposals()[0]);
    expect(f.componentInstance.proposals()[0].status).toBe('pending');
    expect(toasts.show).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
  });

  it('filters by status', async () => {
    const { f } = await render([proposal('a'), proposal('b', { status: 'rejected' })]);
    f.componentInstance.setFilter('all');
    expect(f.componentInstance.shown().length).toBe(2);
    f.componentInstance.setFilter(new CustomEvent('valueChange', { detail: 'rejected' }));
    expect(f.componentInstance.shown().map(p => p.id)).toEqual(['b']);
  });
});

describe('proposal view', () => {
  it('summarises each kind and lists what accepting adds', () => {
    const states = [{ id: 's1', name: 'Start', kind: 'initial' as const }, { id: 's2', name: 'Done', kind: 'final' as const }];
    const transitions = [{ from: 's1', to: 's2', event: 'finish', guard: 'ok' }];
    const elements = proposal('x', { kind: 'states-and-transitions', payload: { states, transitions } });
    expect(summary(elements)).toBe('2 states, 1 transition');
    expect(details(elements)).toContain('Transition Start → Done on "finish" [ok]');
    const model = proposal('m', { kind: 'model', payload: { name: 'Cart', states, transitions } });
    expect(summary(model)).toBe('Model "Cart": 2 states, 1 transition');
    const text = proposal('d', { kind: 'feature-description', payload: { scenarioDescription: 'As a buyer…' } });
    expect(details(text)).toEqual(['As a buyer…']);
    expect(details(proposal('t'))).toEqual(['Case t', 'log in → home']);
    expect(details(proposal('bad', { kind: 'model', payload: null }))).toEqual([]);
  });
});
