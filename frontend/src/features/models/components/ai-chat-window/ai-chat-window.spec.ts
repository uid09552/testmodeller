import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { AiChatStore } from '../../state/ai-chat.store';
import { AiChatComponent } from '../ai-chat/ai-chat';
import { AiChatWindowComponent } from './ai-chat-window';

/** Stands in for the chat itself, whose store talks to the API. */
@Component({ selector: 'tm-ai-chat', template: '<p class="stub-chat">chat</p>' })
class StubChatComponent {}

describe('AiChatWindowComponent', () => {
  let fixture: ReturnType<typeof TestBed.createComponent<AiChatWindowComponent>>;
  let root: HTMLElement;
  const pending = signal(0);

  beforeEach(async () => {
    localStorage.clear();
    pending.set(0);
    await TestBed.configureTestingModule({
      imports: [AiChatWindowComponent],
      providers: [{ provide: AiChatStore, useValue: { pendingCount: pending } }],
    })
      .overrideComponent(AiChatWindowComponent, {
        remove: { imports: [AiChatComponent] },
        add: { imports: [StubChatComponent] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(AiChatWindowComponent);
    root = fixture.nativeElement as HTMLElement;
    fixture.detectChanges();
  });

  const launcher = () => root.querySelector<HTMLButtonElement>('.ai-launcher')!;
  const render = () => { fixture.detectChanges(); };

  it('starts closed, with only the launcher shown', () => {
    expect(launcher()).toBeTruthy();
    expect(root.querySelector('.ai-window')).toBeNull();
  });

  it('opens and closes the window from the launcher', () => {
    launcher().click();
    render();
    expect(root.querySelector('.ai-window .stub-chat')).toBeTruthy();
    expect(launcher().getAttribute('aria-expanded')).toBe('true');

    launcher().click();
    render();
    expect(root.querySelector('.ai-window')).toBeNull();
  });

  it('minimises to the title bar and keeps the chat alive', () => {
    launcher().click();
    render();
    root.querySelector<HTMLButtonElement>('[aria-label="Minimise chat"]')!.click();
    render();
    expect(root.querySelector('.ai-window--min')).toBeTruthy();
    expect(root.querySelector('.ai-window__body--hidden .stub-chat')).toBeTruthy();
  });

  it('shows how many proposals wait for a decision on the launcher', () => {
    pending.set(3);
    render();
    expect(launcher().textContent).toContain('3');
  });

  it('grows when its top-left grip is dragged up and left, within limits', () => {
    const c = fixture.componentInstance;
    c.toggle();
    render();
    c.startResize(new MouseEvent('mousedown', { clientX: 500, clientY: 500 }));
    c.onResizeMove(new MouseEvent('mousemove', { clientX: 450, clientY: 400 }));
    expect(c.width()).toBe(430);
    expect(c.height()).toBe(600);

    c.onResizeMove(new MouseEvent('mousemove', { clientX: -5000, clientY: 5000 }));
    expect(c.width()).toBe(720);
    expect(c.height()).toBe(260);
    c.onResizeEnd();
    expect(JSON.parse(localStorage.getItem('tm:ai-chat-window')!)).toEqual({ w: 720, h: 260 });
  });
});
