import { ChangeDetectionStrategy, Component, output } from '@angular/core';
import { HELP_ROWS } from '../../state/shortcuts';

/** The canvas shortcuts (`?`), listed from the same table the canvas dispatches on. */
@Component({
  selector: 'tm-canvas-help',
  template: `
    <div class="help" role="dialog" aria-label="Keyboard shortcuts">
      <div class="help__head">
        <strong>Keyboard shortcuts</strong>
        <button type="button" class="help__close" aria-label="Close shortcuts" (click)="closed.emit()">×</button>
      </div>
      <table class="shortcuts">
        <tbody>
          @for (s of rows; track s.id) {
            <tr><td><kbd>{{ s.keys }}</kbd></td><td>{{ s.label }}</td></tr>
          }
        </tbody>
      </table>
      <p class="help__note">Tab, arrows, F2, + / − and Ctrl+F work while the canvas has focus.</p>
    </div>`,
  styles: `
    .help {
      width: 340px; max-height: 70vh; overflow-y: auto; padding: 10px 12px;
      background: var(--clr-surface-0); border: 1px solid var(--clr-surface-200);
      border-radius: var(--radius-md); box-shadow: 0 6px 20px rgba(0, 0, 0, .1); font-size: var(--text-xs);
    }
    .help__head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
    .help__close { border: none; background: none; cursor: pointer; font-size: 16px; }
    .help__note { margin-top: 8px; color: var(--clr-text-400); }
    .shortcuts { width: 100%; border-collapse: collapse; }
    td { padding: 3px 4px; vertical-align: top; }
    kbd { font-family: monospace; padding: 1px 5px; border: 1px solid var(--clr-surface-200); border-radius: 4px; white-space: nowrap; }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasHelpComponent {
  readonly rows = HELP_ROWS;
  readonly closed = output<void>();
}
