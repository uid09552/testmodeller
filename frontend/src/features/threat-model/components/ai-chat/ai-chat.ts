import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  TmContextPreview, TmProposal, TmProposalKind,
} from '../../../../core/api/threat-model-api';
import { idsIn, inferKind, KIND_LABEL, KINDS } from '../../state/ai-intent';
import { AiTranscript } from '../../state/ai-transcript';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import { label, levelText, TabSlug, targetName } from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';

/** The tab that edits what a proposal of a kind creates. */
const RESULT_TAB: Record<TmProposalKind, TabSlug> = {
  'tm-architecture': 'components', 'tm-threats': 'threats', 'tm-ratings': 'risks',
  'tm-controls': 'controls', 'tm-security-tests': 'security-tests', 'tm-review': 'overview',
};

type Payload = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

/**
 * The AI assistant of the threat model workspace (FR-085 to FR-089): one
 * message is one request, answered by proposal cards the analyst accepts,
 * edits or rejects. Nothing is applied before that.
 */
@Component({
  selector: 'tm-ai-chat',
  imports: [FormsModule],
  templateUrl: './ai-chat.html',
  styleUrl: './ai-chat.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AiChatComponent {
  readonly store = inject(ThreatModelStore);
  readonly nav = inject(WorkspaceNav);
  readonly transcript = inject(AiTranscript);

  /** An entity named in the URL (`?select=`), offered as the focus. */
  readonly selectedId = input<string | null>(null);

  readonly kinds = KINDS;
  readonly kindLabel = KIND_LABEL;
  readonly label = label;
  readonly levelText = levelText;

  readonly message = signal('');
  readonly kindChoice = signal<TmProposalKind | ''>('');
  readonly focusId = signal('');
  readonly busy = signal(false);
  /** Set while the enable notice is shown. */
  readonly enabling = signal<TmContextPreview | null>(null);
  readonly preview = signal<TmContextPreview | null>(null);
  readonly notice = signal<string | null>(null);
  /** Provider as the server reports it; `none` means nothing can be asked. */
  readonly provider = signal<string | null>(null);
  /** Edited JSON per proposal id, while a card is in edit mode. */
  readonly edits = signal<Record<string, string>>({});
  readonly cardErrors = signal<Record<string, string>>({});

  readonly inferred = computed(() => this.kindChoice() || inferKind(this.message()));

  /** Everything that can be the focus of a request. */
  readonly focusOptions = computed(() => [
    ...this.store.threats().map(t => ({ id: t.id, text: `${t.displayId} ${t.title}`, group: 'Threats' })),
    ...this.store.controls().map(c => ({ id: c.id, text: `${c.displayId} ${c.name}`, group: 'Controls' })),
    ...this.store.components().map(c => ({ id: c.id, text: c.name, group: 'Components' })),
    ...this.store.flows().map(f => ({
      id: f.id, text: targetName({ kind: 'flow', id: f.id }, this.store.components(), this.store.flows()),
      group: 'Flows',
    })),
  ]);
  readonly groups = ['Threats', 'Controls', 'Components', 'Flows'];

  constructor() {
    effect(() => {
      const s = this.selectedId();
      if (s) untracked(() => this.focusId.set(s));
    });
    // Learn the provider once AI is on, so "no provider" is explained before a request fails.
    effect(() => {
      if (this.store.aiEnabled() && this.provider() === null && this.store.id()) {
        untracked(() => void this.loadProvider());
      }
    });
  }

  private async loadProvider(): Promise<void> {
    try {
      this.provider.set((await this.store.previewContext('tm-threats')).provider);
    } catch {
      /* the request itself reports it */
    }
  }

  optionsOf(group: string) {
    return this.focusOptions().filter(o => o.group === group);
  }

  // ── Enabling ───────────────────────────────────────────────────────

  async startEnable(): Promise<void> {
    this.notice.set(null);
    try {
      this.enabling.set(await this.store.previewContext('tm-threats'));
    } catch (e) {
      this.notice.set(errorMessage(e));
    }
  }

  async confirmEnable(): Promise<void> {
    try {
      await this.store.setAiEnabled(true);
      this.provider.set(this.enabling()?.provider ?? null);
      this.enabling.set(null);
    } catch (e) {
      this.notice.set(errorMessage(e));
    }
  }

  async disable(): Promise<void> {
    try {
      await this.store.setAiEnabled(false);
    } catch (e) {
      this.notice.set(errorMessage(e));
    }
  }

  // ── Asking ─────────────────────────────────────────────────────────

  private targets(): string[] {
    const named = idsIn(this.message(), this.store.threats(), this.store.controls(), this.store.tests());
    const focus = this.focusId();
    return [...new Set([...(focus ? [focus] : []), ...named])];
  }

  async showContext(): Promise<void> {
    this.notice.set(null);
    try {
      this.preview.set(await this.store.previewContext(this.inferred(), this.targets()));
    } catch (e) {
      this.notice.set(errorMessage(e));
    }
  }

  async send(): Promise<void> {
    const text = this.message().trim();
    if (!text || this.busy()) return;
    const kind = this.inferred();
    const targets = this.targets();
    this.message.set('');
    this.notice.set(null);
    this.transcript.add({ role: 'user', text, kind });
    const pending = this.transcript.add({ role: 'pending' });
    this.busy.set(true);
    try {
      const proposals = await this.store.requestProposals(kind, text, targets);
      this.transcript.remove(pending);
      this.transcript.add({ role: 'assistant', proposals });
    } catch (e) {
      this.transcript.remove(pending);
      this.transcript.add({ role: 'error', text: this.explain(errorMessage(e)) });
    } finally {
      this.busy.set(false);
    }
  }

  /** The server's reason, with what to do about the usual ones. */
  private explain(msg: string): string {
    if (/provider/i.test(msg)) return `${msg} Check the AI settings.`;
    if (/not enabled/i.test(msg)) return `${msg}`;
    return msg;
  }

  // ── Cards ──────────────────────────────────────────────────────────

  data(proposal: TmProposal): Payload {
    return proposal.payload as Payload;
  }

  isActionable(p: TmProposal): boolean {
    return p.kind !== 'tm-review';
  }

  threatName(id: string): string {
    const t = this.store.threats().find(x => x.id === id);
    return t ? `${t.displayId} ${t.title}` : '(deleted)';
  }

  controlName(id: string): string {
    const c = this.store.controls().find(x => x.id === id);
    return c ? `${c.displayId} ${c.name}` : '(deleted)';
  }

  targetsText(targets: { kind: 'component' | 'flow'; id: string }[]): string {
    return (targets ?? []).map(t => targetName(t, this.store.components(), this.store.flows())).join(', ');
  }

  /** The rating a rating card replaces, as text. */
  oldRating(p: TmProposal): string {
    const t = this.store.threats().find(x => x.id === this.data(p)['threatId']);
    const basis = this.data(p)['rating'] as 'inherent' | 'current' | 'residual';
    const r = t?.[basis];
    return r ? `${r.likelihood} × ${r.impact} = ${r.likelihood * r.impact}` : 'not assessed';
  }

  /** Names a payload uses that are not new: they exist already. */
  existingRefs(p: TmProposal): string[] {
    const a = this.data(p);
    const own = new Set<string>([
      ...(a['zones'] ?? []), ...(a['assets'] ?? []), ...(a['components'] ?? []),
    ].map((x: { name: string }) => x.name.toLowerCase()));
    const refs: string[] = [];
    for (const c of a['components'] ?? []) {
      if (c.zone) refs.push(c.zone);
      refs.push(...(c.assets ?? []));
    }
    for (const f of a['flows'] ?? []) refs.push(f.source, f.target, ...(f.assets ?? []));
    return [...new Set(refs.filter(r => !own.has(r.toLowerCase())))];
  }

  entityLabel(id: string): string {
    const s = this.store;
    return s.threats().find(x => x.id === id)?.displayId
      ?? s.controls().find(x => x.id === id)?.displayId
      ?? s.tests().find(x => x.id === id)?.displayId
      ?? s.components().find(x => x.id === id)?.name
      ?? s.assets().find(x => x.id === id)?.name
      ?? s.zones().find(x => x.id === id)?.name
      ?? (s.flows().some(x => x.id === id) ? targetName({ kind: 'flow', id }, s.components(), s.flows()) : '(deleted)');
  }

  openEntity(id: string): void {
    const s = this.store;
    const tab: TabSlug = s.threats().some(x => x.id === id) ? 'threats'
      : s.controls().some(x => x.id === id) ? 'controls'
      : s.tests().some(x => x.id === id) ? 'security-tests'
      : s.flows().some(x => x.id === id) ? 'data-flows'
      : s.assets().some(x => x.id === id) ? 'assets'
      : s.zones().some(x => x.id === id) ? 'zones' : 'components';
    this.nav.go(tab, { select: id });
  }

  openResult(p: TmProposal): void {
    const id = p.resultingEntityIds[0];
    if (id) this.nav.go(RESULT_TAB[p.kind], { select: id });
  }

  startEdit(p: TmProposal): void {
    this.edits.update(e => ({ ...e, [p.id]: JSON.stringify(p.payload, null, 2) }));
  }

  cancelEdit(p: TmProposal): void {
    this.edits.update(({ [p.id]: _, ...rest }) => rest);
  }

  setEdit(p: TmProposal, text: string): void {
    this.edits.update(e => ({ ...e, [p.id]: text }));
  }

  private cardError(p: TmProposal, text: string | null): void {
    this.cardErrors.update(({ [p.id]: _, ...rest }) => (text ? { ...rest, [p.id]: text } : rest));
  }

  /** Accepts a card; the server is asked first and the workspace changes after it succeeds. */
  async accept(p: TmProposal): Promise<void> {
    this.cardError(p, null);
    let payload: unknown;
    const edited = this.edits()[p.id];
    if (edited !== undefined) {
      try {
        payload = JSON.parse(edited);
      } catch {
        this.cardError(p, 'The edited text is not valid JSON.');
        return;
      }
    }
    try {
      const done = await this.store.acceptProposal(p.id, payload);
      this.transcript.update(done);
      this.cancelEdit(p);
    } catch (e) {
      this.cardError(p, errorMessage(e));
    }
  }

  async reject(p: TmProposal): Promise<void> {
    this.cardError(p, null);
    try {
      this.transcript.update(await this.store.rejectProposal(p.id));
    } catch (e) {
      this.cardError(p, errorMessage(e));
    }
  }
}
