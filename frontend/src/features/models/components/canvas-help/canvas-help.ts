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
      background: var(--si-sys-color-background-1); border: 1px solid var(--si-sys-color-border-3);
      border-radius: var(--si-sys-sizing-border-radius-sm); box-shadow: var(--si-sys-color-effects-shadow-40); font-size: 0.75rem;
    }
    .help__head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
    .help__close { border: none; background: none; cursor: pointer; font-size: 1rem; }
    .help__note { margin-top: 8px; color: var(--si-sys-color-text-secondary); }
    .shortcuts { width: 100%; border-collapse: collapse; }
    td { padding: 3px 4px; vertical-align: top; }
    kbd { font-family: monospace; padding: 1px 5px; border: 1px solid var(--si-sys-color-border-3); border-radius: 4px; white-space: nowrap; }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasHelpComponent {
  readonly rows = HELP_ROWS;
  readonly closed = output<void>();
}
