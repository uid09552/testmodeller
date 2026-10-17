import { NgTemplateOutlet } from '@angular/common';
import {
  afterNextRender, ChangeDetectionStrategy, Component, ElementRef, HostListener, inject, input,
  model, output, TemplateRef,
} from '@angular/core';

/**
 * Side panel holding the editor of one entity. Focus moves to its first
 * field when it opens; Escape cancels.
 */
@Component({
  selector: 'tm-editor-drawer',
  imports: [NgTemplateOutlet],
  templateUrl: './editor-drawer.html',
  styleUrl: './editor-drawer.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class EditorDrawerComponent {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  readonly heading = input.required<string>();
  readonly busy = input(false);
  readonly canDelete = input(false);
  readonly error = input<string | null>(null);
  /** When set, the drawer has two tabs: Details (the form) and Properties (this template). */
  readonly properties = input<TemplateRef<unknown> | null>(null);
  readonly tab = model<'details' | 'properties'>('details');
  readonly save = output<void>();
  readonly closed = output<void>();
  readonly remove = output<void>();

  constructor() {
    afterNextRender(() => {
      this.host.nativeElement
        .querySelector<HTMLElement>('input, select, textarea')
        ?.focus();
    });
  }

  @HostListener('keydown.escape')
  onEscape(): void {
    this.closed.emit();
  }
}
