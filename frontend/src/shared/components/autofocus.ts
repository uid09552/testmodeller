import { afterNextRender, Directive, ElementRef, inject } from '@angular/core';

/** Focuses its element once it has rendered (replaces the `autofocus` attribute). */
@Directive({ selector: '[tmAutofocus]' })
export class AutofocusDirective {
  constructor() {
    const el = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    afterNextRender(() => el.focus());
  }
}
