import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, TemplateRef, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ArchComponent, ArchComponentInput, Criticality } from '../../../../core/api/threat-model-api';
import { ExplorerStore } from '../../../explorer/state/explorer.store';
import { EditorState, orNull } from '../../state/editor-state';
import { ThreatModelStore } from '../../state/threat-model.store';
import { label, TYPE_GROUPS, aiMark } from '../../state/threat-model-view';
import { AiBadgeComponent } from '../ai-badge/ai-badge';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

/** Architecture components (FR-061). Not the project's Components; it may link to one. */
@Component({
  selector: 'tm-components-tab',
  imports: [AiBadgeComponent, FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './components-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ComponentsTabComponent {
  readonly store = inject(ThreatModelStore);
  private readonly explorer = inject(ExplorerStore);
  readonly selectId = input<string | null>(null);
  /** Only the editor, for the diagram: the table is not shown. */
  readonly embedded = input(false);
  /** The diagram's style controls, shown as the drawer's Properties tab. */
  readonly propertiesTpl = input<TemplateRef<unknown> | null>(null);
  readonly drawerTab = signal<'details' | 'properties'>('details');
  readonly groups = TYPE_GROUPS;
  readonly label = label;
  readonly criticalities: Criticality[] = ['low', 'medium', 'high', 'critical'];

  /** The project's Components and Features, for the optional link. */
  readonly projectComponents = computed(() =>
    this.explorer.projects().find(p => p.id === this.store.projectId())?.components ?? []);
  readonly projectFeatures = computed(() =>
    this.projectComponents().flatMap(c => c.features.map(f => ({ id: f.id, name: `${c.name} › ${f.name}` }))));
  readonly assetOptions = computed(() => this.store.assets().map(a => ({ id: a.id, label: a.name })));

  /** "Component › Feature" of a mapped feature, or a dash when none is mapped. */
  featureName(id: string | null | undefined): string {
    return this.projectFeatures().find(f => f.id === id)?.name ?? '—';
  }

  /** Picking a feature also maps the component to the feature's test component, unless one is set. */
  pickFeature(d: ArchComponentInput, featureId: string | null): void {
    d.linkedFeatureId = featureId;
    if (!featureId || d.linkedComponentId) return;
    d.linkedComponentId = this.projectComponents().find(c => c.features.some(f => f.id === featureId))?.id ?? null;
  }

  private zoneName(id: string | null | undefined): string {
    return this.store.zones().find(z => z.id === id)?.name ?? '—';
  }

  readonly columns = computed<Column<ArchComponent>[]>(() => [
    { key: 'name', label: 'Name', text: c => c.name + aiMark(c) },
    { key: 'type', label: 'Type', text: c => label(c.type) },
    { key: 'zone', label: 'Zone', text: c => this.zoneName(c.zoneId) },
    { key: 'exposure', label: 'Exposure', text: c => (c.internetFacing ? 'Internet-facing' : 'Internal') },
    { key: 'criticality', label: 'Criticality', text: c => label(c.criticality) || '—' },
    { key: 'feature', label: 'Feature', text: c => this.featureName(c.linkedFeatureId) },
    { key: 'owner', label: 'Owner', text: c => c.owner ?? '—' },
    { key: 'warnings', label: 'Warnings', text: c => c.warnings.map(w => `⚠ ${w.message}`).join('; '),
      sort: c => c.warnings.length },
  ]);

  readonly editor = new EditorState<ArchComponent, ArchComponentInput>(
    (d, item) => this.store.save('components', {
      ...d,
      description: orNull(d.description), owner: orNull(d.owner), technology: orNull(d.technology),
      zoneId: d.zoneId || null, linkedComponentId: d.linkedComponentId || null,
      linkedFeatureId: d.linkedFeatureId || null, criticality: d.criticality || null,
    }, item ?? undefined),
    c => this.store.remove('components', c.id),
  );

  constructor() {
    effect(() => {
      const id = this.selectId();
      const c = this.store.components().find(x => x.id === id);
      if (c) untracked(() => this.edit(c));
    });
  }

  add(): void {
    this.editor.open(null, {
      name: '', description: '', type: 'service', owner: '', technology: '', criticality: null,
      internetFacing: false, zoneId: null, linkedComponentId: null, linkedFeatureId: null, assetIds: [],
    });
  }

  edit(c: ArchComponent): void {
    this.editor.open(c, {
      name: c.name, description: c.description ?? '', type: c.type, owner: c.owner ?? '',
      technology: c.technology ?? '', criticality: c.criticality ?? null, internetFacing: c.internetFacing,
      zoneId: c.zoneId ?? null, linkedComponentId: c.linkedComponentId ?? null,
      linkedFeatureId: c.linkedFeatureId ?? null, assetIds: [...c.assetIds],
    });
  }

  /** Names how many flows go with the component (spec: delete confirmation). */
  deleteText(c: ArchComponent | null): string {
    if (!c) return '';
    const n = this.store.flows().filter(f => f.sourceId === c.id || f.targetId === c.id).length;
    return n
      ? `Delete component "${c.name}"? ${n} data flow(s) that start or end at it will be deleted too.`
      : `Delete component "${c.name}"?`;
  }
}
