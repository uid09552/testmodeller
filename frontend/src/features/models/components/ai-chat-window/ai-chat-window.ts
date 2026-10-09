import { ChangeDetectionStrategy, Component, HostListener, inject, signal } from '@angular/core';
import { AiChatComponent } from '../ai-chat/ai-chat';
import { AiChatStore } from '../../state/ai-chat.store';
import { readJson, writeJson } from '../../../../core/persistence/local-store';

const SIZE_KEY = 'ai-chat-window';
const WIDTH:  readonly [number, number] = [300, 720];
const HEIGHT: readonly [number, number] = [260, 900];

function clamp(v: number, [min, max]: readonly [number, number]): number {
  return Math.min(Math.max(v, min), max);
}

/**
 * The AI assistant as a chat window floating over the editor's canvas, with a
 * launcher button in its bottom-right corner (docs/specification/06-ui.md).
 *
 * The transcript lives in `AiChatStore`, provided by the editor page, so
 * closing the window keeps the conversation.
 */
@Component({
  selector: 'tm-ai-chat-window',
  imports: [AiChatComponent],
  templateUrl: './ai-chat-window.html',
  styleUrl: './ai-chat-window.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AiChatWindowComponent {
  readonly chat = inject(AiChatStore);

  readonly open = signal(false);
  readonly minimised = signal(false);
  readonly width = signal(380);
  readonly height = signal(500);

  private resizing: { x: number; y: number; w: number; h: number } | null = null;

  constructor() {
    const saved = readJson<{ w: number; h: number }>(SIZE_KEY);
    if (saved) {
      this.width.set(clamp(saved.w, WIDTH));
      this.height.set(clamp(saved.h, HEIGHT));
    }
  }

  toggle(): void {
    this.open.update(v => !v);
    this.minimised.set(false);
  }

  /** The grip is the top-left corner, because the window is anchored bottom-right. */
  startResize(e: MouseEvent): void {
    e.preventDefault();
    this.resizing = { x: e.clientX, y: e.clientY, w: this.width(), h: this.height() };
  }

  @HostListener('document:mousemove', ['$event'])
  onResizeMove(e: MouseEvent): void {
    const r = this.resizing;
    if (!r) return;
    e.preventDefault();
    // Dragging up and to the left enlarges the window.
    this.width.set(clamp(r.w - (e.clientX - r.x), WIDTH));
    this.height.set(clamp(r.h - (e.clientY - r.y), HEIGHT));
  }

  @HostListener('document:mouseup')
  onResizeEnd(): void {
    if (!this.resizing) return;
    this.resizing = null;
    writeJson(SIZE_KEY, { w: this.width(), h: this.height() });
  }
}
