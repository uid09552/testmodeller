import { TestBed } from '@angular/core/testing';
import { TestCaseDialogComponent } from './test-case-dialog';
import { StateTest } from '../../state/model-editor.store';

const TEST: StateTest = {
  id: 't1', seq: 1, name: 'Valid login', category: 'unit', polarity: 'positive',
  given: 'a user', when: 'they log in', then: 'they see home',
};

describe('TestCaseDialogComponent dismissal', () => {
  let fixture: ReturnType<typeof TestBed.createComponent<TestCaseDialogComponent>>;
  let root: HTMLElement;
  let cancels: number;
  let saves: number;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [TestCaseDialogComponent] }).compileComponents();
    fixture = TestBed.createComponent(TestCaseDialogComponent);
    fixture.componentRef.setInput('test', TEST);
    cancels = 0;
    saves = 0;
    fixture.componentInstance.cancelled.subscribe(() => cancels++);
    fixture.componentInstance.save.subscribe(() => saves++);
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  it('stays open when the backdrop is clicked', () => {
    root.querySelector<HTMLElement>('.tcd-backdrop')!.click();
    expect(cancels).toBe(0);
  });

  it('stays open when a drag ends on the backdrop', () => {
    const backdrop = root.querySelector<HTMLElement>('.tcd-backdrop')!;
    root.querySelector('.tcd__input')!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    backdrop.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
    backdrop.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(cancels).toBe(0);
  });

  it('stays open on Escape', () => {
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(cancels).toBe(0);
  });

  it('closes via the X button and the Cancel button', () => {
    root.querySelector<HTMLElement>('.tcd__close')!.click();
    root.querySelector<HTMLElement>('.dialog__btn--cancel')!.click();
    expect(cancels).toBe(2);
  });

  it('saves a valid draft', () => {
    root.querySelector<HTMLElement>('.dialog__btn--confirm')!.click();
    expect(saves).toBe(1);
  });
});
