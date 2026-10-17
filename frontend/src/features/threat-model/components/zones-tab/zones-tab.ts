import {
  ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, TemplateRef, untracked,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Zone, ZoneInput, ZoneKind } from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { ThreatModelStore } from '../../state/threat-model.store';
import { label } from '../../state/threat-model-view';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';

export const ZONE_KINDS: ZoneKind[] = [
  'internet', 'partner', 'dmz', 'internal', 'restricted-data', 'management', 'identity',
  'cicd', 'third-party', 'other',
];

/** Exposure zones and their trust level (FR-062). */
@Component({
  selector: 'tm-zones-tab',
  imports: [FormsModule, EntityTableComponent, EditorDrawerComponent],
  templateUrl: './zones-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ZonesTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly selectId = input<string | null>(null);
  /** Only the editor, for the diagram. */
  readonly embedded = input(false);
  /** The diagram's style controls, shown as the drawer's Properties tab. */
  readonly propertiesTpl = input<TemplateRef<unknown> | null>(null);
  readonly drawerTab = signal<'details' | 'properties'>('details');
  readonly kinds = ZONE_KINDS;
  readonly label = label;
  readonly levels = [0, 1, 2, 3, 4];

  private readonly members = computed(() => {
    const m = new Map<string, number>();
    for (const c of this.store.components()) if (c.zoneId) m.set(c.zoneId, (m.get(c.zoneId) ?? 0) + 1);
    return m;
  });

  readonly columns = computed<Column<Zone>[]>(() => [
    { key: 'name', label: 'Name', text: z => z.name },
    { key: 'kind', label: 'Kind', text: z => label(z.kind) },
    { key: 'trust', label: 'Trust level', text: z => `${z.trustLevel}`, sort: z => z.trustLevel },
    { key: 'members', label: 'Components', text: z => `${this.members().get(z.id) ?? 0}`,
      sort: z => this.members().get(z.id) ?? 0 },
  ]);

  readonly editor = new EditorState<Zone, ZoneInput>(
    (d, item) => this.store.save('zones', { ...d, description: orNull(d.description) }, item ?? undefined),
    z => this.store.remove('zones', z.id),
  );

  constructor() {
    effect(() => {
      const id = this.selectId();
      const z = this.store.zones().find(x => x.id === id);
      if (z) untracked(() => this.edit(z));
    });
  }

  add(): void {
    this.editor.open(null, { name: '', description: '', kind: 'internal', trustLevel: 3 });
  }

  edit(z: Zone): void {
    this.editor.open(z, { name: z.name, description: z.description ?? '', kind: z.kind, trustLevel: z.trustLevel });
  }

  deleteText(z: Zone | null): string {
    const n = z ? this.members().get(z.id) ?? 0 : 0;
    return n ? `Delete zone "${z!.name}"? Its ${n} component(s) will have no zone.` : `Delete zone "${z?.name}"?`;
  }
}
