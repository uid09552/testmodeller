import { CUSTOM_ELEMENTS_SCHEMA, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { IxSelect, IxSelectItem } from '@siemens/ix-angular/standalone';
import { EDITOR_PROJECT_ID, ProjectThreats } from '../../../threat-model/state/project-threats';
import { StateTest } from '../../state/model-editor.store';
import { TestCaseDialogComponent, TestDraft } from './test-case-dialog';

const TEST: StateTest = {
  id: 't1', seq: 1, name: 'Lockout', category: 'feature', polarity: 'negative',
  given: '', when: 'fail three times', then: 'locked', threatIds: ['th1'],
};

describe('TestCaseDialogComponent threats (ADR 0016)', () => {
  async function render(project: string | null, options = [{ id: 'th1', label: 'THR-1 Takeover' }, { id: 'th2', label: 'THR-2 Brute force' }]) {
    const threats = { options: vi.fn(async () => options) };
    TestBed.configureTestingModule({
      imports: [TestCaseDialogComponent],
      providers: [
        { provide: ProjectThreats, useValue: threats },
        { provide: EDITOR_PROJECT_ID, useValue: signal(project) },
      ],
    });
    TestBed.overrideComponent(TestCaseDialogComponent, {
      remove: { imports: [IxSelect, IxSelectItem] },
      add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
    });
    const f = TestBed.createComponent(TestCaseDialogComponent);
    f.componentRef.setInput('test', TEST);
    f.detectChanges();
    await f.whenStable();
    await new Promise(r => setTimeout(r));
    f.detectChanges();
    return { f, threats };
  }

  it("offers the editor project's threats and saves the chosen ones", async () => {
    const { f, threats } = await render('p1');
    expect(threats.options).toHaveBeenCalledWith('p1');
    const select = f.nativeElement.querySelector('ix-select') as HTMLElement;
    expect(select).not.toBeNull();
    expect(f.nativeElement.querySelectorAll('ix-select-item').length).toBe(2);
    expect(f.componentInstance.threatIds()).toEqual(['th1']);

    let saved: TestDraft | undefined;
    f.componentInstance.save.subscribe(d => (saved = d));
    f.componentInstance.setThreats(new CustomEvent('valueChange', { detail: ['th1', 'th2'] }));
    f.componentInstance.onSave();
    expect(saved?.threatIds).toEqual(['th1', 'th2']);
  });

  it('hides the field without a project or without threats', async () => {
    const none = await render(null);
    expect(none.threats.options).not.toHaveBeenCalled();
    expect(none.f.nativeElement.querySelector('ix-select')).toBeNull();
    TestBed.resetTestingModule();
    const empty = await render('p1', []);
    expect(empty.f.nativeElement.querySelector('ix-select')).toBeNull();
  });
});
