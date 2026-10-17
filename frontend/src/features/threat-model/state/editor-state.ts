import { signal } from '@angular/core';
import { errorMessage } from './threat-model.store';

/**
 * The editor of one entity in a tab: which item is open (none for a new one),
 * its draft, and the save/delete round trip. The draft is a plain object the
 * form binds to.
 */
export class EditorState<T extends { id: string; version: number }, D> {
  readonly editing = signal<{ item: T | null; draft: D } | null>(null);
  readonly busy = signal(false);
  readonly error = signal<string | null>(null);

  constructor(
    private readonly persist: (draft: D, item: T | null) => Promise<unknown>,
    private readonly destroy: (item: T) => Promise<unknown>,
  ) {}

  open(item: T | null, draft: D): void {
    this.error.set(null);
    this.editing.set({ item, draft });
  }

  close(): void {
    this.editing.set(null);
    this.error.set(null);
  }

  async save(): Promise<boolean> {
    const e = this.editing();
    if (!e) return false;
    this.busy.set(true);
    this.error.set(null);
    try {
      await this.persist(e.draft, e.item);
      this.close();
      return true;
    } catch (err) {
      this.error.set(errorMessage(err));
      return false;
    } finally {
      this.busy.set(false);
    }
  }

  /** Deletes the open item after `confirmText` is confirmed. */
  async remove(confirmText: string): Promise<void> {
    const item = this.editing()?.item;
    if (!item || !confirm(confirmText)) return;
    this.busy.set(true);
    try {
      await this.destroy(item);
      this.close();
    } catch (err) {
      this.error.set(errorMessage(err));
    } finally {
      this.busy.set(false);
    }
  }
}

/** Blank text to null, for optional fields. */
export function orNull(s: string | null | undefined): string | null {
  return s && s.trim() ? s.trim() : null;
}
