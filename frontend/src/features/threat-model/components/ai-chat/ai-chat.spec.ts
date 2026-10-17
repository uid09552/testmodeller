import { AiTranscript } from '../../state/ai-transcript';
import { AiChatComponent } from './ai-chat';
import {
  component, control, flow, model, proposal, threat,
} from '../../testing/fixtures';
import { byText, mount, setValue } from '../../testing/setup';
import { TestBed } from '@angular/core/testing';

const on = (api: { model: unknown }) => { api.model = model({ aiEnabled: true }); };

async function ask(h: Awaited<ReturnType<typeof mount<AiChatComponent>>>, text: string): Promise<void> {
  setValue(h.root.querySelector<HTMLTextAreaElement>('textarea[name="message"]')!, text);
  await h.settle();
  h.root.querySelector('form')!.dispatchEvent(new Event('submit'));
  await h.settle();
}

describe('AI assistant of the workspace', () => {
  afterEach(() => vi.restoreAllMocks());

  describe('switched off', () => {
    it('explains, shows where details go before enabling, and enables on confirmation', async () => {
      const h = await mount(AiChatComponent);
      expect(h.root.textContent).toContain('AI assistance is off for this threat model');
      expect(h.root.querySelector('textarea[name="message"]')).toBeNull();
      byText(h.root, 'button', 'Enable AI assistance…').click();
      await h.settle();
      const notice = h.root.querySelector('[role="alert"]')!.textContent!;
      expect(notice).toContain('openai-compatible');
      expect(notice).toContain('llm.example.org');
      expect(h.api.writes('update')).toHaveLength(0);
      byText(h.root, 'button', 'Enable AI assistance').click();
      await h.settle();
      expect(h.api.writes('update')[0].args[2]).toEqual(expect.objectContaining({ aiEnabled: true }));
    });

    it('can be cancelled without changing anything', async () => {
      const h = await mount(AiChatComponent);
      byText(h.root, 'button', 'Enable AI assistance…').click();
      await h.settle();
      byText(h.root, 'button', 'Cancel').click();
      await h.settle();
      expect(h.api.writes('update')).toHaveLength(0);
      expect(h.root.textContent).toContain('Enable AI assistance…');
    });
  });

  it('explains when no provider is configured', async () => {
    const h = await mount(AiChatComponent, api => {
      on(api);
      api.previewResult = { ...api.previewResult, provider: 'none', endpointHost: null };
    });
    expect(h.root.textContent).toContain('No AI provider is configured');
    expect(h.root.querySelector('textarea[name="message"]')).toBeNull();
  });

  describe('asking', () => {
    const t3 = threat('t3', 3, { title: 'Spoofed session', targets: [{ kind: 'component', id: 'a' }] });

    function prepare(api: Parameters<Parameters<typeof mount>[1] & object>[0]) {
      on(api);
      api.data.components = [component('a', 'API')];
      api.data.threats = [t3];
      api.data.controls = [control('c1', 1)];
    }

    it('sends one request per message, with the kind read from the text and the named threats as targets', async () => {
      const h = await mount(AiChatComponent, prepare);
      await ask(h, 'suggest controls for THR-3');
      const req = h.api.writes('tmRequest').at(-1)!.args[0] as Record<string, unknown>;
      expect(req).toEqual(expect.objectContaining({ kind: 'tm-controls', targetIds: ['t3'], prompt: 'suggest controls for THR-3' }));
      expect(h.root.querySelector('.aic__msg--user')!.textContent).toContain('Controls');
    });

    it('lets the kind be chosen and the focus be set', async () => {
      const h = await mount(AiChatComponent, prepare);
      setValue(h.root.querySelector<HTMLSelectElement>('select[name="kind"]')!, 'tm-security-tests');
      setValue(h.root.querySelector<HTMLSelectElement>('select[name="focus"]')!, 'a');
      await h.settle();
      await ask(h, 'hello there');
      const req = h.api.writes('tmRequest').at(-1)!.args[0] as Record<string, unknown>;
      expect(req).toEqual(expect.objectContaining({ kind: 'tm-security-tests', targetIds: ['a'] }));
    });

    it('offers the entity named in the URL as the focus', async () => {
      const h = await mount(AiChatComponent, prepare, { selectedId: 't3' });
      expect(h.root.querySelector<HTMLSelectElement>('select[name="focus"]')!.value).toBe('t3');
    });

    it('shows a threat card, accepts it through the server and links to the result', async () => {
      const p = proposal('p1', 'tm-threats', {
        title: 'Tampered request', category: 'tampering', scenario: 'An attacker alters a body.',
        targets: [{ kind: 'component', id: 'a' }],
      });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'what could go wrong?');
      const card = h.root.querySelector('.aic__card')!;
      expect(card.textContent).toContain('Tampered request');
      expect(card.textContent).toContain('API');
      expect(card.textContent).toContain('Proposed by fake-model');
      expect(card.textContent).toContain('Why: because');
      expect(h.api.writes('tmAccept')).toHaveLength(0);

      byText(h.root, 'button', 'Accept').click();
      await h.settle();
      expect(h.api.writes('tmAccept')[0].args).toEqual(['p1', undefined]);
      expect(h.root.querySelector('.aic__card')!.textContent).toContain('Accepted');
      expect(h.root.querySelector('.aic__card .aic__actions')).toBeNull();
    });

    it('rejects a card without applying anything', async () => {
      const p = proposal('p1', 'tm-threats', { title: 'x', category: 'spoofing', targets: [] });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'threats please');
      byText(h.root, 'button', 'Reject').click();
      await h.settle();
      expect(h.api.writes('tmReject')).toHaveLength(1);
      expect(h.api.writes('tmAccept')).toHaveLength(0);
      expect(h.root.querySelector('.aic__card')!.textContent).toContain('Rejected');
    });

    it('accepts an edited payload and refuses invalid JSON', async () => {
      const p = proposal('p1', 'tm-threats', { title: 'Old title', category: 'spoofing', targets: [] });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'threats please');
      byText(h.root, 'button', 'Edit').click();
      await h.settle();
      const ta = h.root.querySelector<HTMLTextAreaElement>('textarea[name="edit-p1"]')!;
      setValue(ta, '{ not json');
      await h.settle();
      byText(h.root, 'button', 'Accept').click();
      await h.settle();
      expect(h.root.querySelector('.aic__card')!.textContent).toContain('not valid JSON');
      expect(h.api.writes('tmAccept')).toHaveLength(0);

      setValue(ta, '{"title":"New title","category":"spoofing","targets":[]}');
      await h.settle();
      byText(h.root, 'button', 'Accept').click();
      await h.settle();
      expect(h.api.writes('tmAccept')[0].args[1]).toEqual({ title: 'New title', category: 'spoofing', targets: [] });
    });

    it('keeps the card pending and shows the reason when the server refuses', async () => {
      const p = proposal('p1', 'tm-threats', { title: 'x', category: 'spoofing', targets: [] });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'threats please');
      h.api.failNext = { status: 422, message: 'targets: the component was deleted' };
      byText(h.root, 'button', 'Accept').click();
      await h.settle();
      const card = h.root.querySelector('.aic__card')!;
      expect(card.textContent).toContain('the component was deleted');
      expect(card.querySelector('.aic__actions')).not.toBeNull();
    });

    it('shows a rating as old and new', async () => {
      const p = proposal('p1', 'tm-ratings', { threatId: 't3', rating: 'current', likelihood: 3, impact: 4 });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.data.threats = [{ ...t3, current: { likelihood: 2, impact: 2 } }];
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'rate THR-3');
      const text = h.root.querySelector('.aic__card')!.textContent!;
      expect(text).toContain('THR-3 Spoofed session');
      expect(text).toContain('2 × 2 = 4');
      expect(text).toContain('3 × 4 = 12');
    });

    it('lists new and already existing elements of an architecture proposal', async () => {
      const p = proposal('p1', 'tm-architecture', {
        zones: [{ name: 'DMZ', kind: 'dmz', trustLevel: 1 }],
        components: [{ name: 'Gateway', type: 'api-gateway', zone: 'DMZ', assets: [] }],
        flows: [{ source: 'Gateway', target: 'API', relationship: 'calls' }],
      });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'we have a gateway in front of the API');
      const text = h.root.querySelector('.aic__card')!.textContent!;
      expect(text).toContain('New zones');
      expect(text).toContain('Gateway → API');
      expect(text).toContain('Uses what already exists');
      expect(text.split('Uses what already exists')[1]).toContain('API');
      expect(text.split('Uses what already exists')[1]).not.toContain('Gateway');
    });

    it('shows a review as text with links and no accept button', async () => {
      const p = proposal('p1', 'tm-review', { findings: [{ text: 'API has no owner', entityIds: ['a'] }] });
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.proposalList = [p];
        api.jobResult = { id: 'job1', status: 'succeeded', proposalIds: ['p1'] };
      });
      await ask(h, 'what is missing in this model?');
      const card = h.root.querySelector('.aic__card')!;
      expect(card.textContent).toContain('API has no owner');
      expect(byText(card as HTMLElement, '.link-btn', 'API')).toBeTruthy();
      expect(card.querySelector('.aic__actions')).toBeNull();
      expect(h.api.writes('tmRequest').at(-1)!.args[0]).toEqual(expect.objectContaining({ kind: 'tm-review' }));
      byText(card as HTMLElement, '.link-btn', 'API').click();
      expect(h.nav.go).toHaveBeenCalledWith('components', { select: 'a' });
    });

    it('shows a failed job in the transcript and says where to look', async () => {
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.jobResult = { id: 'job1', status: 'failed', error: { detail: 'no AI provider configured' } };
      });
      await ask(h, 'what could go wrong?');
      const err = h.root.querySelector('.aic__msg--error')!.textContent!;
      expect(err).toContain('no AI provider configured');
      expect(err).toContain('AI settings');
      expect(h.root.querySelector('.aic__card')).toBeNull();
    });

    it('previews the context and says when it was cut down', async () => {
      const h = await mount(AiChatComponent, api => {
        prepare(api);
        api.previewResult = { context: '{"components":[]}', truncated: true, provider: 'claude', endpointHost: 'api.anthropic.com' };
      });
      setValue(h.root.querySelector<HTMLTextAreaElement>('textarea[name="message"]')!, 'hello');
      await h.settle();
      byText(h.root, 'button', 'Preview context').click();
      await h.settle();
      const pv = h.root.querySelector('[aria-label="Context preview"]')!.textContent!;
      expect(pv).toContain('api.anthropic.com');
      expect(pv).toContain('{"components":[]}');
      expect(pv).toContain('cut down');
      expect(h.api.writes('tmRequest')).toHaveLength(0);
    });

    it('keeps the conversation when the panel is closed and opened again, not after a reload', async () => {
      const h = await mount(AiChatComponent, prepare);
      await ask(h, 'what could go wrong?');
      const transcript = TestBed.inject(AiTranscript);
      expect(transcript.entries().filter(e => e.role === 'user')).toHaveLength(1);
      h.fixture.destroy();
      const again = TestBed.createComponent(AiChatComponent);
      again.detectChanges();
      expect(again.nativeElement.textContent).toContain('what could go wrong?');
      // A new workspace (a reload) has a new transcript.
      expect(new AiTranscript().entries()).toEqual([]);
    });

    it('does not send an empty message', async () => {
      const h = await mount(AiChatComponent, prepare);
      await ask(h, '   ');
      expect(h.api.writes('tmRequest')).toHaveLength(0);
    });
  });

  it('marks AI-created items in the lists', async () => {
    const { aiMark } = await import('../../state/threat-model-view');
    expect(aiMark({ origin: 'ai' })).toBe(' ✦ AI');
    expect(aiMark({ origin: 'manual' })).toBe('');
    expect(aiMark({})).toBe('');
    expect(flow('f', 'a', 'b', { origin: 'ai' }).origin).toBe('ai');
  });
});
