import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from '../../state/model-editor.store';
import { TestCasesPanelComponent } from './test-cases-panel';

describe('TestCasesPanelComponent stale tests', () => {
  it('shows the model count, and a text marker with the reasons on each stale test', async () => {
    await TestBed.configureTestingModule({
      imports: [TestCasesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(TestCasesPanelComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    const stale = store.addTest(a.id, 'Path 1')!;
    store.addTest(a.id, 'Path 2');
    store.select(a.id, 'node');
    store.setStaleTests([{
      testCaseId: stale.id,
      reasons: [{ code: 'STEP_UNASSIGNED', stepOrder: 2, message: 'step 2 lost its transition' }],
    }]);
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;

    expect(root.querySelector('.tests__head .stale-badge')?.textContent?.trim()).toBe('⚠ 1 stale');
    const rows = [...root.querySelectorAll('.tc-row')];
    expect(rows).toHaveLength(2);
    expect(rows[0].querySelector('.stale-badge')?.textContent?.trim()).toBe('⚠ Stale');
    expect(rows[0].querySelector('.stale-reason')?.textContent).toBe('step 2 lost its transition');
    expect(rows[1].querySelector('.stale-badge')).toBeNull();

    (root.querySelector('.tests__head .stale-badge') as HTMLButtonElement).click();
    expect(store.bottomTab()).toBe('validation');
  });
});

describe('TestCasesPanelComponent results', () => {
  it('labels the latest result with text, not colour alone', async () => {
    await TestBed.configureTestingModule({
      imports: [TestCasesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(TestCasesPanelComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    const t = store.addTest(a.id, 'Path 1')!;
    store.addTest(a.id, 'Path 2');
    store.updateTest(a.id, t.id, {
      lastResult: { status: 'failed', executedAt: '2026-10-01T09:00:00Z', message: 'boom' },
    });
    store.select(a.id, 'node');
    fixture.detectChanges();
    const rows = [...(fixture.nativeElement as HTMLElement).querySelectorAll('.tc-row')];

    const badge = rows[0].querySelector('.result-badge')!;
    expect(badge.textContent?.trim()).toBe('✗ Failed');
    expect(badge.classList).toContain('result-badge--failed');
    expect(badge.getAttribute('title')).toContain('boom');
    expect(rows[1].querySelector('.result-badge')).toBeNull();
  });
});
