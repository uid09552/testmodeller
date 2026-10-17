import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { EdgeGeometry, needsLeader } from '../../state/edge-geometry';
import { CanvasEdge, ModelEditorStore } from '../../state/model-editor.store';
import { arrowMarkerId, fontPx, InlineStyle, linePaint, textPaint } from '../../state/style-render';

/** An arrowhead marker the template defines: its kind in one colour. */
export interface ArrowMarker { id: string; kind: 'filled' | 'open' | 'line'; colour: string }

/** Normal sizes of a transition's label and guard text. */
const LABEL_PX = 11;
const GUARD_PX = 10;
const DEFAULT_COLOUR = 'var(--clr-edge)';
const SELECTED_COLOUR = 'var(--clr-brand-500)';

/** A press on one of a transition's handles, for the canvas to turn into a drag. */
export type EdgeGrab =
  | { kind: 'endpoint'; event: MouseEvent; edge: CanvasEdge; end: 'from' | 'to' }
  | { kind: 'bend'; event: MouseEvent; edge: CanvasEdge }
  | { kind: 'waypoint'; event: MouseEvent; edge: CanvasEdge; index: number }
  | { kind: 'label'; event: MouseEvent; edge: CanvasEdge };

/**
 * The visible transitions, drawn above the states, with the handles of the
 * selected one: both ends, the bend handle, the waypoints and the label.
 *
 * Geometry comes from the canvas (`edge-geometry.ts`); drags are the
 * canvas's, because it owns zoom and pan. Rendered inside the canvas SVG
 * through an attribute selector, hence the `svg:` prefixes.
 */
@Component({
  // An element selector cannot work here: the host must be an SVG <g> inside
  // the canvas <svg>, where browsers do not render unknown elements.
  // eslint-disable-next-line @angular-eslint/component-selector
  selector: 'g[tmCanvasEdges]',
  templateUrl: './canvas-edges.html',
  styleUrl: './canvas-edges.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasEdgesComponent {
  readonly store = inject(ModelEditorStore);
  readonly geometries = input.required<Map<string, EdgeGeometry>>();
  /** Present mode: no handles, no dragging. */
  readonly readonly = input(false);
  readonly grab = output<EdgeGrab>();

  readonly needsLeader = needsLeader;

  selected(edge: CanvasEdge): boolean { return this.store.isSelectedId(edge.id); }

  /**
   * Markers for styled transitions, deduplicated by kind and colour. Plain
   * filled arrowheads use the canvas's shared `arrow` / `arrow-selected`.
   */
  readonly markers = computed<ArrowMarker[]>(() => {
    const out = new Map<string, ArrowMarker>();
    const add = (kind: ArrowMarker['kind'], colour: string, key: string) => {
      if (kind === 'filled' && (key === 'default' || key === 'selected')) return;
      const id = arrowMarkerId(kind, key);
      if (!out.has(id)) out.set(id, { id, kind, colour });
    };
    for (const e of this.store.edges()) {
      const kind = e.style?.arrow ?? 'filled';
      const stroke = e.style?.stroke;
      add(kind, stroke ?? DEFAULT_COLOUR, stroke ?? 'default');
      if (this.store.isSelectedId(e.id)) add(kind, SELECTED_COLOUR, 'selected');
    }
    return [...out.values()];
  });

  /** Id of the marker at a transition's target. Selection colours it like the line. */
  markerOf(edge: CanvasEdge): string {
    const kind = edge.style?.arrow ?? 'filled';
    if (this.selected(edge)) return kind === 'filled' ? 'arrow-selected' : arrowMarkerId(kind, 'selected');
    const stroke = edge.style?.stroke;
    if (!stroke && kind === 'filled') return 'arrow';
    return arrowMarkerId(kind, stroke ?? 'default');
  }

  linePaint(edge: CanvasEdge): InlineStyle { return linePaint(edge.style, 1.5); }
  labelPaint(edge: CanvasEdge): InlineStyle { return textPaint(edge.style, LABEL_PX); }

  /** The guard follows its label's colour and size. */
  guardPaint(edge: CanvasEdge): InlineStyle {
    return {
      fill: edge.style?.text ?? null,
      'font-size': edge.style?.size ? `${fontPx(edge.style.size, GUARD_PX)}px` : null,
    };
  }

  /** "2, 5" for a transition taken at steps 2 and 5 of the highlighted path. */
  steps(id: string): string { return (this.store.pathSteps(id) ?? []).join(', '); }

  press(g: EdgeGrab): void {
    if (g.event.button !== 0) return;
    g.event.stopPropagation();
    g.event.preventDefault();
    this.grab.emit(g);
  }

  /** Double-click on the bend handle: straight again. */
  straighten(e: MouseEvent, edge: CanvasEdge): void {
    e.stopPropagation();
    e.preventDefault();
    this.store.straighten(edge.id);
  }

  /** Double-click on a waypoint removes it. */
  removeWaypoint(e: MouseEvent, edge: CanvasEdge, index: number): void {
    e.stopPropagation();
    e.preventDefault();
    this.store.removeWaypoint(edge.id, index);
  }
}
