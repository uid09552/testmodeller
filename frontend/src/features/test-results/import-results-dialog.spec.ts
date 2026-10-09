import { TestBed } from '@angular/core/testing';
import { ApiError } from '../../core/api/api-error';
import { ImportReport, TestResultsApi } from '../../core/api/test-results-api';
import { ImportResultsDialogComponent } from './import-results-dialog';

const REPORT: ImportReport = {
  dryRun: true, total: 45, matched: 40, recorded: 0, duplicates: 0,
  unmatched: ['a', 'b', 'c', 'd'], ambiguous: ['Twice'],
};

describe('ImportResultsDialogComponent', () => {
  let calls: { project: string; format: string; dryRun: boolean; name: string }[];
  let answer: () => Promise<ImportReport>;
  let fixture: ReturnType<typeof TestBed.createComponent<ImportResultsDialogComponent>>;
  let root: HTMLElement;

  beforeEach(async () => {
    calls = [];
    answer = async () => REPORT;
    await TestBed.configureTestingModule({
      imports: [ImportResultsDialogComponent],
      providers: [{
        provide: TestResultsApi,
        useValue: {
          import: (project: string, format: string, file: File, dryRun: boolean) => {
            calls.push({ project, format, dryRun, name: file.name });
            return answer();
          },
        },
      }],
    }).compileComponents();
    fixture = TestBed.createComponent(ImportResultsDialogComponent);
    fixture.componentRef.setInput('projects', [{ id: 'p1', name: 'Shop' }, { id: 'p2', name: 'Bank' }]);
    fixture.componentRef.setInput('projectId', 'p2');
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  const buttons = () => [...root.querySelectorAll<HTMLButtonElement>('.dialog__actions button')];
  const pick = (name: string, size = 10) => {
    const file = new File(['x'.repeat(size)], name);
    fixture.componentInstance.pickFile({ 0: file, length: 1, item: () => file } as unknown as FileList);
    fixture.detectChanges();
  };
  const settle = async () => { await fixture.whenStable(); fixture.detectChanges(); };

  it('cannot send before a file is chosen', () => {
    expect(buttons()[1].disabled).toBe(true);
    expect(buttons()[2].disabled).toBe(true);
  });

  it('guesses the format from the file name', () => {
    pick('cucumber-report.json');
    expect(fixture.componentInstance.format()).toBe('cucumber');
    pick('TEST-login.xml');
    expect(fixture.componentInstance.format()).toBe('junit');
  });

  it('previews with a dry run into the preselected project and shows the report', async () => {
    pick('results.xml');
    buttons()[1].click();
    await settle();

    expect(calls).toEqual([{ project: 'p2', format: 'junit', dryRun: true, name: 'results.xml' }]);
    const text = root.querySelector('.import__report')!.textContent!.replace(/\s+/g, ' ');
    expect(text).toContain('Preview: 45 results in the file, 40 matched, 40 would be recorded');
    expect(text).toContain('Unmatched (4)');
    expect(text).toContain('Twice');
  });

  it('imports and tells the caller when something was recorded', async () => {
    answer = async () => ({ ...REPORT, dryRun: false, recorded: 40 });
    let imported = 0;
    fixture.componentInstance.imported.subscribe(() => imported++);
    pick('results.xml');
    buttons()[2].click();
    await settle();

    expect(calls[0].dryRun).toBe(false);
    expect(imported).toBe(1);
    expect(root.querySelector('.import__report')!.textContent).toContain('40 recorded');
  });

  it('shows the server explanation when the file is rejected', async () => {
    answer = async () => { throw new ApiError('cannot import this file: XML with a DOCTYPE is not accepted', 400); };
    pick('evil.xml');
    buttons()[2].click();
    await settle();

    expect(root.querySelector('[role="alert"]')!.textContent).toContain('DOCTYPE');
    expect(root.querySelector('.import__report')).toBeNull();
  });

  it('refuses a file over 10 MB before sending it', () => {
    pick('huge.xml', 10 * 1024 * 1024 + 1);
    expect(root.querySelector('[role="alert"]')!.textContent).toContain('10 MB');
    expect(buttons()[2].disabled).toBe(true);
  });
});

describe('ImportResultsDialogComponent keyboard', () => {
  it('closes on Escape', async () => {
    await TestBed.configureTestingModule({
      imports: [ImportResultsDialogComponent],
      providers: [{ provide: TestResultsApi, useValue: {} }],
    }).compileComponents();
    const fixture = TestBed.createComponent(ImportResultsDialogComponent);
    fixture.componentRef.setInput('projects', [{ id: 'p1', name: 'Shop' }]);
    fixture.detectChanges();
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => closed++);
    fixture.nativeElement.querySelector('[role="dialog"]')
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(closed).toBe(1);
  });
});
