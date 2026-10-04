import {
  ChangeDetectionStrategy, Component, computed, effect, HostListener,
  inject, OnInit, signal,
} from '@angular/core';
import { Router, RouterLink, ActivatedRoute } from '@angular/router';
import { CanvasComponent } from '../components/canvas/canvas';
import { PropertiesPanelComponent } from '../components/properties-panel/properties-panel';
import { BottomPanelComponent } from '../components/bottom-panel/bottom-panel';
import { ModelEditorStore } from '../state/model-editor.store';
import { ModelRepository } from '../state/model-repository';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { readJson, writeJson } from '../../../core/persistence/local-store';

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max);
}

@Component({
  selector: 'tm-model-editor-page',
  imports: [RouterLink, CanvasComponent, PropertiesPanelComponent, BottomPanelComponent],
  providers: [ModelEditorStore],   // each editor instance gets its own store
  templateUrl: './model-editor-page.html',
  styleUrl: './model-editor-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ModelEditorPageComponent implements OnInit {
  readonly store = inject(ModelEditorStore);
  private readonly repo     = inject(ModelRepository);
  private readonly explorer = inject(ExplorerStore);
  private readonly route    = inject(ActivatedRoute);
  private readonly router   = inject(Router);

  // Panel collapse — gives the canvas full width/height on demand.
  readonly propsCollapsed  = signal(false);
  readonly bottomCollapsed = signal(false);

  // ── Resizable panels ───────────────────────────────────────────────────────
  private static readonly PROPS_MIN = 200;
  private static readonly PROPS_MAX = 620;
  private static readonly BOTTOM_MIN = 90;
  private static readonly BOTTOM_MAX = 680;
  private static readonly LAYOUT_KEY = 'editor-layout';

  readonly propsWidth   = signal(280);
  readonly bottomHeight = signal(180);

  /** Which splitter is being dragged, with its starting geometry. */
  private resizing: { axis: 'x' | 'y'; start: number; origin: number } | null = null;

  // Bound as custom properties rather than grid-template so the responsive
  // breakpoint in the stylesheet can still restructure the grid (an inline
  // grid-template would override it). Collapse is folded in here because an
  // inline value also beats a class rule.
  readonly propsWidthPx = computed(() =>
    `${this.propsCollapsed() ? 0 : this.propsWidth()}px`);

  readonly bottomHeightPx = computed(() =>
    `${this.bottomCollapsed() ? 37 : this.bottomHeight()}px`);

  startResizeProps(e: MouseEvent): void {
    e.preventDefault();
    this.resizing = { axis: 'x', start: e.clientX, origin: this.propsWidth() };
  }

  startResizeBottom(e: MouseEvent): void {
    e.preventDefault();
    this.resizing = { axis: 'y', start: e.clientY, origin: this.bottomHeight() };
  }

  @HostListener('document:mousemove', ['$event'])
  onResizeMove(e: MouseEvent): void {
    const r = this.resizing;
    if (!r) return;
    e.preventDefault();
    if (r.axis === 'x') {
      // Dragging left widens the panel, so the delta is inverted.
      const next = r.origin - (e.clientX - r.start);
      this.propsWidth.set(clamp(next, ModelEditorPageComponent.PROPS_MIN,
                                      ModelEditorPageComponent.PROPS_MAX));
    } else {
      const next = r.origin - (e.clientY - r.start);
      this.bottomHeight.set(clamp(next, ModelEditorPageComponent.BOTTOM_MIN,
                                        ModelEditorPageComponent.BOTTOM_MAX));
    }
  }

  @HostListener('document:mouseup')
  onResizeEnd(): void {
    if (!this.resizing) return;
    this.resizing = null;
    writeJson(ModelEditorPageComponent.LAYOUT_KEY, {
      props: this.propsWidth(), bottom: this.bottomHeight(),
    });
  }

  /** Double-clicking a splitter restores its default size. */
  resetPropsWidth():   void { this.propsWidth.set(280);   this.onResizeEnd(); }
  resetBottomHeight(): void { this.bottomHeight.set(180); this.onResizeEnd(); }

  readonly saving        = signal(false);
  readonly savedAt       = signal<Date | null>(null);
  readonly confirmDelete = signal(false);

  /** Feature context passed from the Explorer. */
  readonly featureName = signal<string | null>(null);
  /** The id this editor persists under. */
  private readonly modelId = signal<string | null>(null);

  constructor() {
    // Autosave: any change to model content is written through to the
    // repository, so nothing is lost on refresh or navigation.
    effect(() => {
      const nodes = this.store.nodes();
      const edges = this.store.edges();
      const name  = this.store.name();
      this.store.scenarioDesc();
      this.store.description();
      this.store.modelStatus();

      const id = this.modelId();
      if (!id) return;

      this.repo.save(this.store.toPersisted(id));
      this.explorer.syncModelStats(id, {
        name,
        states: nodes.length,
        transitions: edges.length,
      });
    });
  }

  ngOnInit(): void {
    const saved = readJson<{ props: number; bottom: number }>(
      ModelEditorPageComponent.LAYOUT_KEY,
    );
    if (saved) {
      this.propsWidth.set(clamp(saved.props, ModelEditorPageComponent.PROPS_MIN,
                                             ModelEditorPageComponent.PROPS_MAX));
      this.bottomHeight.set(clamp(saved.bottom, ModelEditorPageComponent.BOTTOM_MIN,
                                                ModelEditorPageComponent.BOTTOM_MAX));
    }

    const qp = this.route.snapshot.queryParamMap;
    const routeId = this.route.snapshot.paramMap.get('modelId');
    this.featureName.set(qp.get('feature-name'));

    // `/models/new` gets a fresh id so autosave has somewhere to write.
    const id = routeId ?? crypto.randomUUID();

    const stored = this.repo.get(id);
    if (stored) {
      this.store.loadFrom(stored);
    } else {
      this.store.reset();
      this.store.setName(qp.get('name') ?? 'Untitled Model');
      this.store.clearDirty();
    }

    this.modelId.set(id);
  }

  async save(): Promise<void> {
    this.saving.set(true);
    const id = this.modelId();
    if (id) this.repo.save(this.store.toPersisted(id));
    // TODO: replace with POST/PUT /models once the API contract is wired up.
    await new Promise(r => setTimeout(r, 300));
    this.store.clearDirty();
    this.savedAt.set(new Date());
    this.saving.set(false);
  }

  toggleProps():  void { this.propsCollapsed.update(v => !v); }
  toggleBottom(): void { this.bottomCollapsed.update(v => !v); }

  requestDelete(): void { this.confirmDelete.set(true); }
  cancelDelete():  void { this.confirmDelete.set(false); }

  async confirmDeleteModel(): Promise<void> {
    const id = this.modelId();
    this.confirmDelete.set(false);
    if (id) {
      this.modelId.set(null);   // stop autosave re-creating the record
      this.repo.remove(id);
    }
    // TODO: replace with DELETE /models/{id}
    await this.router.navigate(['/explorer']);
  }
}
