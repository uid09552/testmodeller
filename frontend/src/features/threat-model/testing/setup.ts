import { signal, Type } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { ThreatModelApi } from '../../../core/api/threat-model-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { ThreatModelIndex } from '../state/threat-model-index';
import { ThreatModelStore } from '../state/threat-model.store';
import { WorkspaceNav } from '../state/workspace-nav';
import { FakeThreatModelApi } from './fixtures';

export interface Harness<T> {
  fixture: ComponentFixture<T>;
  root: HTMLElement;
  api: FakeThreatModelApi;
  store: ThreatModelStore;
  nav: { go: ReturnType<typeof vi.fn> };
  settle(): Promise<void>;
}

export const PROJECTS = [{
  id: 'p1', name: 'Shop', expanded: true,
  components: [{
    id: 'c1', name: 'Payments', expanded: true,
    features: [{ id: 'f1', name: 'Pay', description: '', models: [], expanded: true }],
  }],
}];

/**
 * Mounts a tab with a real store over the fake API. `prepare` fills the
 * fake's data before the store loads.
 */
export async function mount<T>(
  type: Type<T>, prepare: (api: FakeThreatModelApi) => void = () => undefined, inputs: Record<string, unknown> = {},
): Promise<Harness<T>> {
  const api = new FakeThreatModelApi();
  prepare(api);
  const nav = { go: vi.fn(), projectId: 'p1' };
  TestBed.configureTestingModule({
    imports: [type],
    providers: [
      provideRouter([]),
      ThreatModelStore,
      { provide: WorkspaceNav, useValue: nav },
      { provide: ThreatModelApi, useValue: api },
      { provide: ThreatModelIndex, useValue: { set: () => undefined } },
      { provide: ExplorerStore, useValue: { projects: signal(PROJECTS) } },
    ],
  });
  const store = TestBed.inject(ThreatModelStore);
  await store.load('p1');
  const fixture = TestBed.createComponent(type);
  for (const [k, v] of Object.entries(inputs)) fixture.componentRef.setInput(k, v);
  fixture.detectChanges();
  // Writes reload the threat model, which takes several promise turns.
  const settle = async () => {
    for (let i = 0; i < 3; i++) {
      await new Promise(r => setTimeout(r));
      await fixture.whenStable();
      fixture.detectChanges();
    }
  };
  await settle();
  return { fixture, root: fixture.nativeElement, api, store, nav, settle };
}

export function byText<E extends HTMLElement = HTMLButtonElement>(root: HTMLElement, selector: string, text: string): E {
  const el = [...root.querySelectorAll<E>(selector)].find(e => e.textContent?.trim().includes(text));
  if (!el) throw new Error(`no ${selector} with text "${text}"`);
  return el;
}

export function setValue(el: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement, value: string): void {
  el.value = value;
  el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? 'change' : 'input'));
}
