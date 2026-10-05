import {
  ChangeDetectionStrategy, Component, ElementRef, computed, effect, inject,
  signal, viewChild,
} from '@angular/core';
import { ProposalKind } from '../../../../core/api/api.types';
import { ChatCard, ChatEntry, AiChatStore } from '../../state/ai-chat.store';
import { ChatIntent, kindLabel } from '../../state/ai-chat.mapping';
import {
  ElementsPayload, FeatureDescriptionPayload, ModelInput, TestCaseInput,
} from '../../../../core/api/api.types';

/** What the composer offers, in the order it is shown. */
const INTENTS: { value: ChatIntent; label: string }[] = [
  { value: 'auto',                   label: 'Auto' },
  { value: 'test-cases',             label: 'Test cases' },
  { value: 'feature-description',    label: 'Scenario' },
  { value: 'states-and-transitions', label: 'States' },
  { value: 'model',                  label: 'Model' },
];

/** Ready-made first messages, so the panel is usable without a blank page. */
const SUGGESTIONS = [
  'Propose negative test cases for the selected state',
  'Write a scenario description for this feature',
  'Suggest the states and transitions this flow is missing',
];

/**
 * Chat panel for the AI assistant in the model editor.
 *
 * The transcript lives in `AiChatStore`; this component renders it and the
 * composer. Each proposal is a card the user accepts or rejects — nothing is
 * applied to the model on its own (FR-033).
 */
@Component({
  selector: 'tm-ai-chat',
  templateUrl: './ai-chat.html',
  styleUrl: './ai-chat.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AiChatComponent {
  readonly store = inject(AiChatStore);

  readonly intents = INTENTS;
  readonly suggestions = SUGGESTIONS;

  readonly draft = signal('');

  private readonly transcript = viewChild<ElementRef<HTMLElement>>('transcript');

  readonly canSend = computed(() =>
    this.draft().trim().length > 0 && !this.store.working());

  /** The kind the current draft would be sent as, shown next to the button. */
  readonly previewKind = computed(() => {
    const text = this.draft().trim();
    if (!text) return null;
    return kindLabel(this.store.resolvedKind(text));
  });

  constructor() {
    // Keep the newest entry in view as the turn progresses.
    effect(() => {
      this.store.entries();
      this.store.working();
      setTimeout(() => {
        const el = this.transcript()?.nativeElement;
        if (el) el.scrollTop = el.scrollHeight;
      });
    });
  }

  setIntent(value: ChatIntent): void { this.store.intent.set(value); }

  use(suggestion: string): void { this.draft.set(suggestion); }

  async send(): Promise<void> {
    if (!this.canSend()) return;
    const text = this.draft();
    this.draft.set('');
    await this.store.send(text);
  }

  /** Enter sends; Shift+Enter inserts a newline. */
  onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void this.send();
    }
  }

  // ── Card rendering ────────────────────────────────────────────────────────
  cardTitle(card: ChatCard): string {
    const p = card.proposal;
    switch (p.kind) {
      case 'test-cases':
        return (p.payload as TestCaseInput).name ?? 'Test case';
      case 'feature-description':
        return 'Scenario description';
      case 'model':
        return (p.payload as ModelInput).name ?? 'Model';
      case 'states-and-transitions':
        return 'States & transitions';
    }
  }

  kindLabel(kind: ProposalKind): string { return kindLabel(kind); }

  /** A short, readable digest of the payload, so the user can judge it. */
  cardSummary(card: ChatCard): string[] {
    const p = card.proposal;
    switch (p.kind) {
      case 'test-cases': {
        const tc = p.payload as TestCaseInput;
        const lines: string[] = [];
        if (tc.preconditions) lines.push(`Given ${tc.preconditions}`);
        for (const s of tc.steps ?? []) {
          if (s.action) lines.push(`When ${s.action}`);
          if (s.expected) lines.push(`Then ${s.expected}`);
        }
        return lines.length ? lines : ['No steps.'];
      }
      case 'feature-description':
        return [(p.payload as FeatureDescriptionPayload).scenarioDescription ?? ''];
      case 'states-and-transitions':
      case 'model': {
        const g = p.payload as ElementsPayload;
        const states = (g.states ?? []).map(s => `${s.name} (${s.kind})`);
        const transitions = (g.transitions ?? []).map(t => `${t.from} → ${t.to}: ${t.event}`);
        return [...states, ...transitions];
      }
    }
  }

  isCard(entry: ChatEntry): entry is ChatCard { return entry.kind === 'card'; }

  accept(card: ChatCard): void { void this.store.accept(card); }
  reject(card: ChatCard): void { void this.store.reject(card); }
}
