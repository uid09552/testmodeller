import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { EdgeGeometry, needsLeader } from '../../state/edge-geometry';
import { CanvasEdge, ModelEditorStore } from '../../state/model-editor.store';

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
