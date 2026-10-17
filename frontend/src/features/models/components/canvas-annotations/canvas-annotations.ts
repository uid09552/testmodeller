import {
  ChangeDetectionStrategy, Component, ElementRef, inject, input, output, signal, viewChild,
} from '@angular/core';
import { labelLineH, LABEL_FONT_PX, singleLine } from '../../state/node-fit';
import {
  ANNOTATION_MAX_TEXT, CanvasAnnotation, ModelEditorStore, NOTE_PAD, ResizeHandle,
} from '../../state/model-editor.store';
import { InlineStyle, shapePaint, textPaint } from '../../state/style-render';

/** A press on an annotation or one of its handles, for the canvas to turn into a drag. */
export type AnnotationGrab =
  | { kind: 'move'; event: MouseEvent; annotation: CanvasAnnotation }
  | { kind: 'resize'; event: MouseEvent; annotation: CanvasAnnotation; handle: ResizeHandle };

/** Size of a note's folded corner. */
const FOLD = 14;

/**
 * Notes and text boxes on the canvas: drawn above the groups and below the
 * states, with resize handles and an in-place text editor. They are not part
 * of the model (see `CanvasAnnotation`).
 *
 * Drags are the canvas's, because it owns zoom and pan. Rendered inside the
 * canvas SVG through an attribute selector, like `canvas-edges`.
 */
@Component({
  // The host must be an SVG <g> inside the canvas <svg> (see canvas-edges).
  // eslint-disable-next-line @angular-eslint/component-selector
  selector: 'g[tmCanvasAnnotations]',
  templateUrl: './canvas-annotations.html',
  styleUrl: './canvas-annotations.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasAnnotationsComponent {
  readonly store = inject(ModelEditorStore);
  /** Present mode: no editing, moving or resizing. */
  readonly readonly = input(false);
  readonly grab = output<AnnotationGrab>();

  readonly handles: ResizeHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];
  readonly pad = NOTE_PAD;
  readonly maxText = ANNOTATION_MAX_TEXT;

  /** The annotation whose text is being edited. */
  readonly editId = signal<string | null>(null);
  /** Set when the last input was cut at the length limit. */
  readonly cut = signal(false);
  private original = '';
  private readonly editInput = viewChild<ElementRef<HTMLTextAreaElement>>('editInput');

  selected(a: CanvasAnnotation): boolean { return this.store.isSelectedId(a.id); }

  lines(a: CanvasAnnotation): string[] { return this.store.annotationLines(a); }
  lineH(a: CanvasAnnotation): number { return labelLineH(a.style); }
  /** Baseline of the first line, one padding below the top. */
  firstBaseline(a: CanvasAnnotation): number { return NOTE_PAD + this.lineH(a) * 0.8; }

  /** A note's outline: a box with its top-right corner folded. */
  notePath(a: CanvasAnnotation): string {
    return `M0,0 L${a.w - FOLD},0 L${a.w},${FOLD} L${a.w},${a.h} L0,${a.h} Z`;
  }
  foldPath(a: CanvasAnnotation): string {
    return `M${a.w - FOLD},0 L${a.w - FOLD},${FOLD} L${a.w},${FOLD}`;
  }

  bodyPaint(a: CanvasAnnotation): InlineStyle { return shapePaint(a.style, 1); }
  textPaint(a: CanvasAnnotation): InlineStyle { return textPaint(a.style, LABEL_FONT_PX); }

  ariaLabel(a: CanvasAnnotation): string {
    return `${a.kind === 'note' ? 'Note' : 'Text'}: ${singleLine(a.text) || '(empty)'}`;
  }

  /** Where a resize handle sits, in the annotation's coordinates. */
  handlePos(a: CanvasAnnotation, h: ResizeHandle): { x: number; y: number } {
    const x = h.includes('w') ? 0 : h.includes('e') ? a.w : a.w / 2;
    const y = h.includes('n') ? 0 : h.includes('s') ? a.h : a.h / 2;
    return { x, y };
  }

  press(e: MouseEvent, a: CanvasAnnotation): void {
    if (e.button !== 0 || this.editId() === a.id) return;
    this.grab.emit({ kind: 'move', event: e, annotation: a });
  }

  pressHandle(e: MouseEvent, a: CanvasAnnotation, handle: ResizeHandle): void {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    this.grab.emit({ kind: 'resize', event: e, annotation: a, handle });
  }

  onDblClick(e: MouseEvent, a: CanvasAnnotation): void {
    e.stopPropagation();
    if (this.readonly()) return;
    this.edit(a.id);
  }

  // ── In-place editor ───────────────────────────────────────────────────
  /** Opens the text editor; editing an existing text is one undo step. */
  edit(id: string): void {
    const a = this.store.annotationById(id);
    if (!a || this.readonly()) return;
    if (a.text) this.store.checkpoint();
    this.original = a.text;
    this.cut.set(false);
    this.editId.set(id);
    // Focus once the textarea exists, so blur and the keys work.
    setTimeout(() => {
      const el = this.editInput()?.nativeElement;
      if (!el) return;
      // Set once: binding the value would rewrite it on every keystroke.
      el.value = a.text;
      el.focus();
      el.select();
    });
  }

  onInput(ta: HTMLTextAreaElement): void {
    const id = this.editId();
    if (!id) return;
    const cut = this.store.setAnnotationText(id, ta.value);
    this.cut.set(cut);
    if (cut) ta.value = ta.value.slice(0, ANNOTATION_MAX_TEXT);
  }

  onKeydown(e: KeyboardEvent, ta: HTMLTextAreaElement): void {
    // Shift+Enter falls through to the textarea and inserts a line break.
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); this.finish(ta.value); }
    if (e.key === 'Escape') { e.preventDefault(); this.finish(this.original); }
  }

  /** Commits `text`; an empty annotation is removed. */
  finish(text: string): void {
    const id = this.editId();
    if (!id) return;
    this.editId.set(null);
    this.cut.set(false);
    this.store.commitAnnotationText(id, text);
  }
}
