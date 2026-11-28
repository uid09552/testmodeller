import { CUSTOM_ELEMENTS_SCHEMA } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  IxActiveModal, IxButton, IxInput, IxMessageBar, IxModalContent, IxModalFooter, IxModalHeader,
  IxNumberInput, IxSelect, IxSelectItem, IxSpinner,
} from '@siemens/ix-angular/standalone';
import { ApiError } from '../../../../core/api/api-error';
import { TestCase } from '../../../../core/api/api.types';
import { OrgApi } from '../../../../core/api/org-api';
import { GenerateDialogComponent } from './generate-dialog';
import { diffGeneration } from './generation-diff';

const tc = (name: string, steps: [string, string][] = [['a', 'b']], extra: Partial<TestCase> = {}): TestCase => ({
  id: name, version: 1, featureId: 'f', name, assignments: [],
  steps: steps.map(([action, expected]) => ({ action, expected })), ...extra,
});

const COVERAGE = { states: { covered: 3, total: 4 }, transitions: { covered: 2, total: 2 } };

describe('diffGeneration', () => {
  it('sorts names into new, changed, unchanged and dropped', () => {
    const before = [tc('A'), tc('B'), tc('C')];
    const after = [tc('A'), tc('B', [['x', 'y']]), tc('D')];
    expect(diffGeneration(before, after)).toEqual({
      added: ['D'], unchanged: ['A'], changed: ['B'], dropped: ['C'],
    });
  });
});

// Importing iX registers its custom elements, which upgrade in jsdom and
// observe visibility, an API jsdom does not have.
class NoIntersectionObserver {
  observe(): void { /* jsdom has no layout */ }
  unobserve(): void { /* nothing observed */ }
  disconnect(): void { /* nothing observed */ }
  takeRecords(): [] { return []; }
}

describe('GenerateDialogComponent', () => {
  beforeAll(() => {
    (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver ??= NoIntersectionObserver;
  });

  async function render(existing: TestCase[] = []) {
    const api = {
      modelTestCases: vi.fn(async () => existing),
      generate: vi.fn(async (_: string, req: { save?: boolean }) => ({
        seed: 42, testCases: [tc('Path 1'), tc('Path 2')], coverage: COVERAGE,
        ...(req.save ? {} : {}),
      })),
    };
    const modal = { data: { modelId: 'm1', modelName: 'Login' }, close: vi.fn(), dismiss: vi.fn() };
    TestBed.configureTestingModule({
      imports: [GenerateDialogComponent],
      providers: [{ provide: OrgApi, useValue: api }, { provide: IxActiveModal, useValue: modal }],
    });
    TestBed.overrideComponent(GenerateDialogComponent, {
      remove: { imports: [IxButton, IxInput, IxMessageBar, IxModalContent, IxModalFooter, IxModalHeader, IxNumberInput, IxSelect, IxSelectItem, IxSpinner] },
      add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
    });
    const f = TestBed.createComponent(GenerateDialogComponent);
    f.detectChanges();
    await f.whenStable();
    f.detectChanges();
    return { f, c: f.componentInstance, api, modal, root: f.nativeElement as HTMLElement };
  }

  it('previews without saving, then saves exactly what was previewed', async () => {
    const { f, c, api, modal, root } = await render();
    await c.preview();
    f.detectChanges();
    const sent = api.generate.mock.calls.at(-1)![1] as { seed: number };
    expect(sent).toEqual({ criterion: 'transition', seed: expect.any(Number), save: false });
    expect(Number.isSafeInteger(sent.seed)).toBe(true);
    expect(root.textContent).toContain('2 test cases');
    expect(root.textContent).toContain('3 of 4 states (75 %)');
    // Save repeats the preview's seed, so the stored set is the one shown.
    await c.save();
    expect(api.generate).toHaveBeenLastCalledWith('m1', { criterion: 'transition', seed: sent.seed, save: true });
    expect(modal.close).toHaveBeenCalledWith({ saved: [expect.objectContaining({ name: 'Path 1' }), expect.anything()] });
  });

  it('sends the path length only for bounded paths, and checks the inputs', async () => {
    const { c, api } = await render();
    c.setCriterion(new CustomEvent('valueChange', { detail: 'bounded-paths' }));
    c.setMaxPathLength(new CustomEvent('valueChange', { detail: 0 }));
    expect(c.canRun()).toBe(false);
    c.setMaxPathLength(new CustomEvent('valueChange', { detail: 5 }));
    c.setSeed(new CustomEvent('valueChange', { detail: 'abc' }));
    expect(c.canRun()).toBe(false);
    c.setSeed(new CustomEvent('valueChange', { detail: '7' }));
    await c.preview();
    expect(api.generate).toHaveBeenLastCalledWith('m1', { criterion: 'bounded-paths', seed: 7, maxPathLength: 5, save: false });
  });

  it('compares with the existing generated test cases and warns about duplicates', async () => {
    const { f, c, root } = await render([
      tc('Path 1', [['a', 'b']], { origin: 'generated', generatedFromModelId: 'm1' }),
      tc('Old', [['a', 'b']], { origin: 'generated', generatedFromModelId: 'm1' }),
      tc('Manual', [['a', 'b']], { origin: 'manual' }),
    ]);
    await c.preview();
    f.detectChanges();
    expect(root.textContent).toContain('1 new, 0 changed, 1 unchanged, 1 no longer generated');
    expect(root.querySelector('ix-message-bar[type="warning"]')!.textContent).toContain('Saving adds all 2 again');
  });

  it('shows why generation failed and keeps the dialog open', async () => {
    const { f, c, api, modal, root } = await render();
    api.generate.mockRejectedValueOnce(new ApiError('the model has no initial state', 422));
    await c.save();
    f.detectChanges();
    expect(root.textContent).toContain('the model has no initial state');
    expect(modal.close).not.toHaveBeenCalled();
    c.cancel();
    expect(modal.dismiss).toHaveBeenCalledWith(null);
  });
});
