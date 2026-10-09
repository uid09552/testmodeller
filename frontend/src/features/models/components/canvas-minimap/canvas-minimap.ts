import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { Box, boundsOf, fromMinimap, minimapTransform, View, visibleBox } from '../../state/viewport';

export const MINIMAP_W = 180;
export const MINIMAP_H = 120;

/**
 * Overview of the whole model with the visible area marked. Clicking or
 * dragging in it pans the canvas so that point is in the middle.
 */
@Component({
  selector: 'tm-canvas-minimap',
  templateUrl: './canvas-minimap.html',
  styleUrl: './canvas-minimap.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasMinimapComponent {
  readonly boxes = input.required<Box[]>();
  readonly view = input.required<View>();
  readonly viewSize = input.required<{ w: number; h: number }>();
  /** Canvas point to centre the view on. */
  readonly panTo = output<{ x: number; y: number }>();

  readonly w = MINIMAP_W;
  readonly h = MINIMAP_H;
  private readonly dragging = signal(false);

  /** What the minimap shows: the model and the visible area together. */
  private readonly world = computed(() => {
    const { w, h } = this.viewSize();
    const visible = w && h ? [visibleBox(this.view(), w, h)] : [];
    return boundsOf([...this.boxes(), ...visible]) ?? { x: 0, y: 0, w: 1, h: 1 };
  });
  readonly t = computed(() => minimapTransform(this.world(), MINIMAP_W, MINIMAP_H));
  readonly viewport = computed(() => {
    const { w, h } = this.viewSize();
    if (!w || !h) return null;
    const b = visibleBox(this.view(), w, h);
    const t = this.t();
    return { x: b.x * t.scale + t.ox, y: b.y * t.scale + t.oy, w: b.w * t.scale, h: b.h * t.scale };
  });

  private at(e: MouseEvent): void {
    const r = (e.currentTarget as Element).getBoundingClientRect();
    this.panTo.emit(fromMinimap({ x: e.clientX - r.left, y: e.clientY - r.top }, this.t()));
  }

  down(e: MouseEvent): void {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    this.dragging.set(true);
    this.at(e);
  }

  move(e: MouseEvent): void { if (this.dragging()) this.at(e); }
  up(): void { this.dragging.set(false); }
}
