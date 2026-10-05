import {
  ChangeDetectionStrategy, Component, computed, effect, HostListener, inject, signal,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  ExplorerStore, ExplorerProject, ExplorerComponent, ExplorerFeature, ExplorerModel,
  NewItemKind,
} from '../../state/explorer.store';

type CtxTarget =
  | { kind: 'project';   project: ExplorerProject }
  | { kind: 'component'; project: ExplorerProject; component: ExplorerComponent }
  | { kind: 'feature';   project: ExplorerProject; component: ExplorerComponent; feature: ExplorerFeature }
  | { kind: 'model';     project: ExplorerProject; component: ExplorerComponent; feature: ExplorerFeature; model: ExplorerModel };

interface ContextMenu { x: number; y: number; target: CtxTarget }

/**
 * The Project > Component > Feature > Model tree.
 *
 * Rendered inside the nav bar (ADR 0004), so it is mounted once for the whole
 * app: selection lives in `ExplorerStore` and the Explorer page reads it back.
 */
@Component({
  selector: 'tm-explorer-tree',
  templateUrl: './explorer-tree.html',
  styleUrl: './explorer-tree.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExplorerTreeComponent {
  readonly store = inject(ExplorerStore);
  private readonly router = inject(Router);

  // ── Inline rename ──────────────────────────────────────────────────────────
  readonly editId    = signal<string | null>(null);
  readonly editValue = signal('');

  readonly ctxMenu = signal<ContextMenu | null>(null);

  // ── New item dialog ────────────────────────────────────────────────────────
  // The request comes from the store, so the Explorer page's buttons open the
  // same dialog this component renders.
  readonly newPrompt = this.store.newItemPrompt;
  readonly newValue = signal('');

  readonly newPromptTitle = computed(() => {
    const k = this.newPrompt()?.kind;
    return k ? k.charAt(0).toUpperCase() + k.slice(1) : '';
  });

  constructor() {
    // The dialog can be opened from elsewhere (the Explorer page), so the input
    // is cleared here rather than at every call site.
    effect(() => {
      if (this.newPrompt()) this.newValue.set('');
    });
  }

  // ── Tree selection ─────────────────────────────────────────────────────────
  // Selecting a container also toggles it, and shows its detail on the
  // Explorer page — the tree itself is always visible in the nav.
  selectProject(project: ExplorerProject): void {
    this.store.select({ kind: 'project', projectId: project.id });
    this.store.toggleProject(project.id);
    void this.router.navigate(['/explorer']);
  }

  selectComponent(project: ExplorerProject, component: ExplorerComponent): void {
    this.store.select({
      kind: 'component', projectId: project.id, componentId: component.id,
    });
    this.store.toggleComponent(project.id, component.id);
    void this.router.navigate(['/explorer']);
  }

  selectFeature(
    project: ExplorerProject, component: ExplorerComponent, feature: ExplorerFeature,
  ): void {
    this.store.select({
      kind: 'feature', projectId: project.id, componentId: component.id,
      featureId: feature.id,
    });
    this.store.toggleFeature(project.id, component.id, feature.id);
    void this.router.navigate(['/explorer']);
  }

  /** A single click on a model opens the editor — there is no overview step. */
  selectModel(
    project: ExplorerProject, component: ExplorerComponent,
    feature: ExplorerFeature, model: ExplorerModel,
  ): void {
    this.store.select({
      kind: 'model', projectId: project.id, componentId: component.id,
      featureId: feature.id, modelId: model.id,
    });
    this.openModel(model, feature);
  }

  /** Open a model in the editor. The feature id travels with it: the AI
   *  endpoints are scoped to a feature and the editor cannot look it up. */
  openModel(model: ExplorerModel, feature: ExplorerFeature): void {
    void this.router.navigate(['/models', model.id]);
  }

  /** Create a brand-new (empty) model for a feature. */
  createModel(
    project: ExplorerProject, component: ExplorerComponent, feature: ExplorerFeature,
  ): void {
    // featureName is captured here: clicking "+" does not change the selection,
    // so it cannot be read back off the store later.
    this.store.promptNew({
      kind: 'model', projectId: project.id, componentId: component.id,
      featureId: feature.id, featureName: feature.name,
    });
    this.newValue.set('');
  }

  // ── Context menu ──────────────────────────────────────────────────────────
  openCtx(e: MouseEvent, target: CtxTarget): void {
    e.preventDefault();
    e.stopPropagation();
    // Clamp to the viewport — the menu is position:fixed.
    const x = Math.max(8, Math.min(e.clientX, window.innerWidth  - 210));
    const y = Math.max(8, Math.min(e.clientY, window.innerHeight - 220));
    this.ctxMenu.set({ x, y, target });
  }

  closeCtx(): void { this.ctxMenu.set(null); }

  @HostListener('document:mousedown', ['$event'])
  onDocMouseDown(e: MouseEvent): void {
    if (this.ctxMenu() && !(e.target as Element).closest('.ctx-menu')) this.closeCtx();
  }

  @HostListener('document:keydown.escape')
  onEscape(): void { this.closeCtx(); }

  runCtxAction(action: string): void {
    const m = this.ctxMenu();
    this.closeCtx();
    if (!m) return;
    const t = m.target;

    switch (action) {
      case 'rename':
        if (t.kind === 'project')   this.startEdit(t.project.id,   t.project.name);
        if (t.kind === 'component') this.startEdit(t.component.id, t.component.name);
        if (t.kind === 'feature')   this.startEdit(t.feature.id,   t.feature.name);
        if (t.kind === 'model')     this.startEdit(t.model.id,     t.model.name);
        break;
      case 'add-component':
        if (t.kind === 'project') this.openNewPrompt('component', t.project.id);
        break;
      case 'add-feature':
        if (t.kind === 'component') this.openNewPrompt('feature', t.project.id, t.component.id);
        break;
      case 'add-model':
        if (t.kind === 'feature') this.createModel(t.project, t.component, t.feature);
        break;
      case 'open-model':
        if (t.kind === 'model') this.openModel(t.model, t.feature);
        break;
      case 'delete':
        if (t.kind === 'project')   this.store.deleteProject(t.project.id);
        if (t.kind === 'component') this.store.deleteComponent(t.project.id, t.component.id);
        if (t.kind === 'feature')   this.store.deleteFeature(t.project.id, t.component.id, t.feature.id);
        if (t.kind === 'model')     this.store.deleteModel(t.project.id, t.component.id, t.feature.id, t.model.id);
        this.store.select(null);
        break;
    }
  }

  // ── Inline rename ─────────────────────────────────────────────────────────
  startEdit(id: string, current: string): void {
    this.editId.set(id);
    this.editValue.set(current);
  }

  finishEdit(
    p?: ExplorerProject, c?: ExplorerComponent, f?: ExplorerFeature, m?: ExplorerModel,
  ): void {
    const id  = this.editId();
    const val = this.editValue().trim();
    if (!id || !val) { this.editId.set(null); return; }

    if (m && f && c && p)      this.store.renameModel(p.id, c.id, f.id, m.id, val);
    else if (f && c && p)      this.store.renameFeature(p.id, c.id, f.id, val);
    else if (c && p)           this.store.renameComponent(p.id, c.id, val);
    else if (p)                this.store.renameProject(p.id, val);
    this.editId.set(null);
  }

  onEditKey(
    e: KeyboardEvent,
    p?: ExplorerProject, c?: ExplorerComponent, f?: ExplorerFeature, m?: ExplorerModel,
  ): void {
    if (e.key === 'Enter')  { e.preventDefault(); this.finishEdit(p, c, f, m); }
    if (e.key === 'Escape') { e.preventDefault(); this.editId.set(null); }
  }

  // ── New item dialog ────────────────────────────────────────────────────────
  openNewPrompt(
    kind: NewItemKind, projectId?: string, componentId?: string, featureId?: string,
  ): void {
    this.store.promptNew({ kind, projectId, componentId, featureId });
    this.newValue.set('');
  }

  confirmNew(): void {
    void this.create();
  }

  private async create(): Promise<void> {
    const p = this.newPrompt();
    const name = this.newValue().trim();
    if (!p || !name) { this.store.closePrompt(); return; }
    this.store.closePrompt();

    if (p.kind === 'project') await this.store.addProject(name);
    if (p.kind === 'component' && p.projectId)
      await this.store.addComponent(p.projectId, name);
    if (p.kind === 'feature' && p.projectId && p.componentId)
      await this.store.addFeature(p.projectId, p.componentId, name);
    if (p.kind === 'model' && p.projectId && p.componentId && p.featureId) {
      const model = await this.store.addModel(p.projectId, p.componentId, p.featureId, name);
      if (!model) return;
      this.store.select({
        kind: 'model', projectId: p.projectId, componentId: p.componentId,
        featureId: p.featureId, modelId: model.id,
      });
      void this.router.navigate(['/models', model.id]);
    }
  }

  cancelNew(): void { this.store.closePrompt(); }

  isEditing(id: string): boolean { return this.editId() === id; }
}
