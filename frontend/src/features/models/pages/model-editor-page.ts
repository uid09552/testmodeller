import {
  ChangeDetectionStrategy, Component, computed, effect, HostListener,
  inject, OnInit, signal, untracked,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { Router, RouterLink, ActivatedRoute } from '@angular/router';
import { CanvasComponent } from '../components/canvas/canvas';
import { PropertiesPanelComponent } from '../components/properties-panel/properties-panel';
import { TestCasesPanelComponent } from '../components/test-cases-panel/test-cases-panel';
import { BottomPanelComponent } from '../components/bottom-panel/bottom-panel';
import { AiChatComponent } from '../components/ai-chat/ai-chat';
import { AiChatStore } from '../state/ai-chat.store';
import { ModelEditorStore } from '../state/model-editor.store';
import { ModelPersistenceService } from '../state/model-persistence';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { readJson, writeJson } from '../../../core/persistence/local-store';

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max);
}

/** The panes of the editor's right column, in the order they are shown. */
export type SideTab = 'tests' | 'properties' | 'chat';

@Component({
  selector: 'tm-model-editor-page',
  imports: [
    RouterLink, CanvasComponent, TestCasesPanelComponent, PropertiesPanelComponent,
    BottomPanelComponent, AiChatComponent,
  ],
  // Each editor instance gets its own stores. The chat store is here, not in
  // the panel, so the transcript survives switching the right-panel tab.
  providers: [ModelEditorStore, ModelPersistenceService, AiChatStore],
  templateUrl: './model-editor-page.html',
  styleUrl: './model-editor-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ModelEditorPageComponent implements OnInit {
  readonly store = inject(ModelEditorStore);
  readonly chat  = inject(AiChatStore);
  private readonly remote   = inject(ModelPersistenceService);
  private readonly explorer = inject(ExplorerStore);
  private readonly route    = inject(ActivatedRoute);
  private readonly router   = inject(Router);

  // Panel collapse — gives the canvas full width/height on demand.
  readonly propsCollapsed  = signal(false);
  readonly bottomCollapsed = signal(false);

  /**
   * Which pane the right column shows (docs/specification/06-ui.md). Test
   * cases are the default: they are what the tool is for.
   */
  readonly sideTab = signal<SideTab>('tests');

  showTab(tab: SideTab): void {
    this.sideTab.set(tab);
    this.propsCollapsed.set(false);
  }

  /**
   * The canvas asked to show a state's test cases: open the right column on the
   * Test Cases tab, then let the panel scroll them into view.
   */
  revealTests(): void {
    this.showTab('tests');
    this.store.focusTests();
  }

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

  /** The feature the open model belongs to, as the database has it. */
  readonly featureName = this.remote.featureName;
  readonly featureId   = this.remote.featureId;
  /** What the stored copy is up to; the template shows failures. */
  readonly saveState   = this.remote.state;
  readonly saveError   = this.remote.error;
  /** The id this editor persists under; null while loading, so autosave is off. */
  private readonly modelId = signal<string | null>(null);

  /** Route id the editor is currently showing, so a re-run is a no-op. */
  private readonly loadedRouteId = signal<string | null | undefined>(undefined);

  // Route parameters as signals: they emit synchronously, so the first value
  // is available before the first render.
  private readonly routeParams = toSignal(this.route.paramMap, { requireSync: true });

  constructor() {
    // Follow the route: opening another model from the tree reuses this
    // component, so this is what loads it.
    effect(() => {
      const routeId = this.routeParams().get('modelId');
      // Only the route is a dependency; everything the load touches is read
      // and written outside the reactive context.
      untracked(() => this.openRouteModel(routeId));
    });

    // Autosave: any change to model content is scheduled for storage.
    effect(() => {
      const nodes = this.store.nodes();
      const edges = this.store.edges();
      const name  = this.store.name();
      this.store.scenarioDesc();
      this.store.description();
      this.store.modelStatus();

      const id = this.modelId();
      if (!id) return;

      this.remote.schedule(this.store.toPersisted(id));
      this.explorer.syncModelStats(id, {
        name,
        status: this.store.modelStatus(),
        states: nodes.length,
        transitions: edges.length,
      });
    });

    effect(() => {
      if (this.remote.state() === 'saved') this.store.clearDirty();
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
  }

  /**
   * Loads the model the route points at.
   *
   * Picking another model in the tree only changes the route parameter, and
   * Angular reuses this component for that, so the load has to follow the
   * route rather than run once on init — otherwise the editor keeps showing
   * the model that was open before.
   */
  private openRouteModel(routeId: string | null): void {
    if (routeId === this.loadedRouteId()) return;
    this.loadedRouteId.set(routeId);

    // Stop autosave first, so nothing is saved against the wrong model while
    // the new one loads.
    this.modelId.set(null);
    this.store.reset();
    this.store.clearDirty();
    this.savedAt.set(null);
    this.confirmDelete.set(false);

    // The transcript goes with the old model: its proposals belong to that feature.
    this.chat.clear();

    if (!routeId) {
      void this.router.navigate(['/explorer']);
      return;
    }
    this.chat.setContext({ localModelId: routeId, modelName: null });

    // Autosave stays off until the stored copy is here, or an empty canvas
    // could be saved over it.
    void this.remote.open(routeId).then(loaded => {
      if (this.loadedRouteId() !== routeId || !loaded) return;   // moved on, or failed
      this.store.loadFrom(loaded);
      this.modelId.set(routeId);
    });
  }

  /** After a conflict: take the other version. */
  async takeTheirs(): Promise<void> {
    const loaded = await this.remote.takeTheirs();
    if (loaded) this.store.loadFrom(loaded);
  }

  keepMine(): Promise<void> { return this.remote.keepMine(); }

  async save(): Promise<void> {
    this.saving.set(true);
    const id = this.modelId();
    if (id) {
      this.remote.schedule(this.store.toPersisted(id));
      const stored = await this.remote.flush();
      if (stored) {
        this.store.clearDirty();
        this.savedAt.set(new Date());
      }
    }
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
      this.modelId.set(null);   // stop autosave
      const path = this.explorer.modelPaths().find(p => p.model.id === id);
      try {
        await this.remote.remove();
      } catch {
        return;   // not deleted: stay, the model is still there
      }
      if (path) this.explorer.removeModel(path.projectId, path.componentId, path.featureId, id);
    }
    await this.router.navigate(['/explorer']);
  }
}
