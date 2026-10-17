import {
  ChangeDetectionStrategy, Component, HostListener, inject, input, signal,
} from '@angular/core';
import { readJson, writeJson } from '../../../../core/persistence/local-store';
import { ThreatModelStore } from '../../state/threat-model.store';
import { AiChatComponent } from '../ai-chat/ai-chat';

const SIZE_KEY = 'tm-ai-chat-window';
const WIDTH: readonly [number, number] = [320, 760];
const HEIGHT: readonly [number, number] = [280, 900];

function clamp(v: number, [min, max]: readonly [number, number]): number {
  return Math.min(Math.max(v, min), max);
}

/**
 * The AI assistant of the threat model workspace as a chat window floating
 * over the page, with a launcher button in the bottom-right corner, as in the
 * model editor (docs/specification/06-ui.md).
 *
 * The transcript lives in `AiTranscript`, provided by the workspace page, so
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
  readonly store = inject(ThreatModelStore);
  /** An entity named in the URL, offered to the chat as its focus. */
  readonly selectedId = input<string | null>(null);

  readonly open = signal(false);
  readonly minimised = signal(false);
  readonly width = signal(400);
  readonly height = signal(560);

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

  /** The grip is the top-right corner, because the window is anchored bottom-left. */
  startResize(e: MouseEvent): void {
    e.preventDefault();
    this.resizing = { x: e.clientX, y: e.clientY, w: this.width(), h: this.height() };
  }

  @HostListener('document:mousemove', ['$event'])
  onResizeMove(e: MouseEvent): void {
    const r = this.resizing;
    if (!r) return;
    e.preventDefault();
    this.width.set(clamp(r.w + (e.clientX - r.x), WIDTH));
    this.height.set(clamp(r.h - (e.clientY - r.y), HEIGHT));
  }

  @HostListener('document:mouseup')
  onResizeEnd(): void {
    if (!this.resizing) return;
    this.resizing = null;
    writeJson(SIZE_KEY, { w: this.width(), h: this.height() });
  }
}
