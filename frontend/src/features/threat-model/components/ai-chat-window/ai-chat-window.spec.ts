import { AiChatWindowComponent } from './ai-chat-window';
import { model, proposal } from '../../testing/fixtures';
import { mount, setValue } from '../../testing/setup';

describe('AI assistant window', () => {
  it('floats over the page: a launcher opens a window, which closes again', async () => {
    const h = await mount(AiChatWindowComponent);
    expect(h.root.querySelector('.ai-window')).toBeNull();
    const launcher = h.root.querySelector<HTMLButtonElement>('.ai-launcher')!;
    expect(launcher.getAttribute('aria-expanded')).toBe('false');
    launcher.click();
    await h.settle();
    expect(h.root.querySelector('.ai-window')!.getAttribute('aria-label')).toBe('AI assistant');
    expect(h.root.querySelector('tm-ai-chat')).not.toBeNull();
    h.root.querySelector<HTMLButtonElement>('[aria-label="Close chat"]')!.click();
    await h.settle();
    expect(h.root.querySelector('.ai-window')).toBeNull();
  });

  it('keeps the conversation when it is closed and opened again', async () => {
    const h = await mount(AiChatWindowComponent, api => { api.model = model({ aiEnabled: true }); });
    h.root.querySelector<HTMLButtonElement>('.ai-launcher')!.click();
    await h.settle();
    setValue(h.root.querySelector<HTMLTextAreaElement>('textarea[name="message"]')!, 'what could go wrong?');
    await h.settle();
    h.root.querySelector('form')!.dispatchEvent(new Event('submit'));
    await h.settle();
    h.root.querySelector<HTMLButtonElement>('[aria-label="Close chat"]')!.click();
    await h.settle();
    h.root.querySelector<HTMLButtonElement>('.ai-launcher')!.click();
    await h.settle();
    expect(h.root.textContent).toContain('what could go wrong?');
  });

  it('minimises to its title bar and shows how many proposals wait', async () => {
    const h = await mount(AiChatWindowComponent, api => {
      api.proposalList = [proposal('p1', 'tm-threats', {}), proposal('p2', 'tm-ratings', {})];
    });
    expect(h.root.querySelector('.ai-launcher .ai-badge')!.textContent).toContain('2');
    h.root.querySelector<HTMLButtonElement>('.ai-launcher')!.click();
    await h.settle();
    h.root.querySelector<HTMLButtonElement>('[aria-label="Minimise chat"]')!.click();
    await h.settle();
    expect(h.root.querySelector('.ai-window--min')).not.toBeNull();
    expect(h.root.querySelector('.ai-window__body--hidden')).not.toBeNull();
  });
});
