import { Injectable, signal } from '@angular/core';
import { TmProposal, TmProposalKind } from '../../../core/api/threat-model-api';

export type Entry =
  | { id: number; role: 'user'; text: string; kind: TmProposalKind }
  | { id: number; role: 'assistant'; proposals: TmProposal[] }
  | { id: number; role: 'error' | 'info'; text: string }
  | { id: number; role: 'pending' };

/** An entry before it has an id (`Omit` would collapse the union). */
export type NewEntry = Entry extends infer E ? (E extends Entry ? Omit<E, 'id'> : never) : never;

/**
 * The chat transcript of the open workspace. It lives in the browser only
 * (ADR 0003): nothing earlier is sent with a new request, and a reload starts
 * empty, while pending proposals stay available from the server.
 */
@Injectable()
export class AiTranscript {
  readonly entries = signal<Entry[]>([]);
  private seq = 0;

  add(e: NewEntry): number {
    const id = ++this.seq;
    this.entries.update(list => [...list, { ...e, id } as Entry]);
    return id;
  }

  remove(id: number): void {
    this.entries.update(list => list.filter(e => e.id !== id));
  }

  /** Replaces a proposal wherever it is shown. */
  update(p: TmProposal): void {
    this.entries.update(list => list.map(e =>
      e.role === 'assistant' ? { ...e, proposals: e.proposals.map(x => (x.id === p.id ? p : x)) } : e));
  }
}
