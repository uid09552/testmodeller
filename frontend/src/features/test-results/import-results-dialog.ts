import {
  ChangeDetectionStrategy, Component, computed, inject, input, output, signal,
} from '@angular/core';
import { ApiError } from '../../core/api/api-error';
import { AutofocusDirective } from '../../shared/components/autofocus';
import {
  formatFromName, ImportReport, MAX_RESULT_FILE_BYTES, ResultFormat, TestResultsApi,
} from '../../core/api/test-results-api';

/**
 * Imports a JUnit XML or Cucumber JSON result file into a project (FR-050):
 * pick a file, preview the matching (dry run), then import. See
 * docs/specification/06-ui.md#test-results.
 */
@Component({
  selector: 'tm-import-results-dialog',
  imports: [AutofocusDirective],
  templateUrl: './import-results-dialog.html',
  styleUrl: './import-results-dialog.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportResultsDialogComponent {
  private readonly api = inject(TestResultsApi);

  readonly projects = input.required<{ id: string; name: string }[]>();
  /** Preselected project; the first one otherwise. */
  readonly projectId = input<string | null>(null);
  readonly closed = output<void>();
  /** Results were recorded; the caller reloads. */
  readonly imported = output<ImportReport>();

  private readonly chosenProject = signal<string | null>(null);
  readonly project = computed(() =>
    this.chosenProject() ?? this.projectId() ?? this.projects()[0]?.id ?? null);
  readonly format = signal<ResultFormat>('junit');
  readonly file = signal<File | null>(null);
  readonly busy = signal(false);
  readonly error = signal<string | null>(null);
  readonly report = signal<ImportReport | null>(null);

  readonly canSend = computed(() => !!this.project() && !!this.file() && !this.busy());

  setProject(id: string): void { this.chosenProject.set(id); this.report.set(null); }

  setFormat(f: string): void {
    if (f === 'junit' || f === 'cucumber') this.format.set(f);
    this.report.set(null);
  }

  pickFile(files: FileList | null): void {
    const f = files?.[0] ?? null;
    this.report.set(null);
    this.error.set(null);
    if (f && f.size > MAX_RESULT_FILE_BYTES) {
      this.file.set(null);
      this.error.set('The file is larger than 10 MB.');
      return;
    }
    this.file.set(f);
    const guessed = f ? formatFromName(f.name) : null;
    if (guessed) this.format.set(guessed);
  }

  preview(): Promise<void> { return this.send(true); }
  run(): Promise<void> { return this.send(false); }

  private async send(dryRun: boolean): Promise<void> {
    const project = this.project();
    const file = this.file();
    if (!project || !file) return;
    this.busy.set(true);
    this.error.set(null);
    try {
      const report = await this.api.import(project, this.format(), file, dryRun);
      this.report.set(report);
      if (!dryRun && report.recorded > 0) this.imported.emit(report);
    } catch (e) {
      this.report.set(null);
      this.error.set(e instanceof ApiError ? e.message : 'The file could not be imported.');
    } finally {
      this.busy.set(false);
    }
  }

  close(): void { this.closed.emit(); }
}
