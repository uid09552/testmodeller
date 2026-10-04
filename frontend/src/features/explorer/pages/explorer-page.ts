import {
  ChangeDetectionStrategy, Component, inject, signal, computed,
} from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import {
  ExplorerStore, ExplorerProject, ExplorerComponent, ExplorerFeature, ExplorerModel,
} from '../state/explorer.store';

type CtxTarget =
  | { kind: 'project';   project: ExplorerProject }
  | { kind: 'component'; project: ExplorerProject; component: ExplorerComponent }
  | { kind: 'feature';   project: ExplorerProject; component: ExplorerComponent; feature: ExplorerFeature }
  | { kind: 'model';     project: ExplorerProject; component: ExplorerComponent; feature: ExplorerFeature; model: ExplorerModel };

interface ContextMenu { x: number; y: number; target: CtxTarget }

type NewKind = 'project' | 'component' | 'feature' | 'model';

@Component({
  selector: 'tm-explorer-page',
  imports: [RouterLink],
  templateUrl: './explorer-page.html',
  styleUrl:    './explorer-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExplorerPageComponent {
  readonly store  = inject(ExplorerStore);
  private readonly router = inject(Router);

  readonly selected = signal<CtxTarget | null>(null);

  // ── Inline rename ──────────────────────────────────────────────────────────
  readonly editId    = signal<string | null>(null);
  readonly editValue = signal('');

  readonly ctxMenu = signal<ContextMenu | null>(null);

  // ── New item dialog ────────────────────────────────────────────────────────
  readonly newPrompt = signal<{
    kind: NewKind; projectId?: string; componentId?: string;
    featureId?: string; featureName?: string;
  } | null>(null);
  readonly newValue = signal('');

  readonly newPromptTitle = computed(() => {
    const k = this.newPrompt()?.kind;
    return k ? k.charAt(0).toUpperCase() + k.slice(1) : '';
  });

  // ── Selected-level accessors ───────────────────────────────────────────────
  readonly selProject   = computed(() => this.selected()?.project   ?? null);
  readonly selComponent = computed(() => {
    const s = this.selected();
    return s && s.kind !== 'project' ? s.component : null;
  });
  readonly selFeature = computed(() => {
    const s = this.selected();
    return s && (s.kind === 'feature' || s.kind === 'model') ? s.feature : null;
  });
  readonly selModel = computed(() => {
    const s = this.selected();
    return s?.kind === 'model' ? s.model : null;
  });

  // ── Tree selection ─────────────────────────────────────────────────────────
  selectProject(project: ExplorerProject): void {
    this.selected.set({ kind: 'project', project });
    this.store.toggleProject(project.id);
  }

  selectComponent(project: ExplorerProject, component: ExplorerComponent): void {
    this.selected.set({ kind: 'component', project, component });
    this.store.toggleComponent(project.id, component.id);
  }

  selectFeature(project: ExplorerProject, component: ExplorerComponent, feature: ExplorerFeature): void {
    this.selected.set({ kind: 'feature', project, component, feature });
    this.store.toggleFeature(project.id, component.id, feature.id);
  }

  selectModel(
    project: ExplorerProject, component: ExplorerComponent,
    feature: ExplorerFeature, model: ExplorerModel,
  ): void {
    this.selected.set({ kind: 'model', project, component, feature, model });
  }

  /** Open an existing model in the editor. */
  openModel(model: ExplorerModel, feature: ExplorerFeature): void {
    this.router.navigate(['/models', model.id], {
      queryParams: { name: model.name, 'feature-name': feature.name },
    });
  }

  /** Create a brand-new (empty) model for a feature. */
  createModel(project: ExplorerProject, component: ExplorerComponent, feature: ExplorerFeature): void {
    // featureName is captured here: clicking "+" does not change the selection,
    // so it cannot be read back off selFeature() later.
    this.newPrompt.set({
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

  onDocClick(e: MouseEvent): void {
    if (!(e.target as Element).closest('.ctx-menu')) this.closeCtx();
  }

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
        this.selected.set(null);
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
  openNewPrompt(kind: NewKind, projectId?: string, componentId?: string, featureId?: string): void {
    this.newPrompt.set({ kind, projectId, componentId, featureId });
    this.newValue.set('');
  }

  confirmNew(): void {
    const p = this.newPrompt();
    const name = this.newValue().trim();
    if (!p || !name) { this.newPrompt.set(null); return; }

    if (p.kind === 'project') this.store.addProject(name);
    if (p.kind === 'component' && p.projectId)
      this.store.addComponent(p.projectId, name);
    if (p.kind === 'feature' && p.projectId && p.componentId)
      this.store.addFeature(p.projectId, p.componentId, name);
    if (p.kind === 'model' && p.projectId && p.componentId && p.featureId) {
      const model = this.store.addModel(p.projectId, p.componentId, p.featureId, name);
      this.newPrompt.set(null);
      // Navigate to the model's real id so the editor's autosave writes
      // against the same record the tree points at. Nothing is stored for it
      // yet, so the editor opens on an empty canvas.
      this.router.navigate(['/models', model.id], {
        queryParams: { name: model.name, 'feature-name': p.featureName ?? '' },
      });
      return;
    }
    this.newPrompt.set(null);
  }

  cancelNew(): void { this.newPrompt.set(null); }

  isEditing(id: string): boolean { return this.editId() === id; }

  isSelected(id: string): boolean {
    const s = this.selected();
    if (!s) return false;
    if (s.kind === 'project')   return s.project.id   === id;
    if (s.kind === 'component') return s.component.id === id;
    if (s.kind === 'feature')   return s.feature.id   === id;
    return s.model.id === id;
  }
}
