import { ChangeDetectionStrategy, Component, computed, effect, inject, input, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Asset, AssetCategory, AssetInput, CiaLevel } from '../../../../core/api/threat-model-api';
import { EditorState, orNull } from '../../state/editor-state';
import { componentInput, flowInput } from '../../state/inputs';
import { ThreatModelStore } from '../../state/threat-model.store';
import { label, targetName } from '../../state/threat-model-view';
import { EditorDrawerComponent } from '../editor-drawer/editor-drawer';
import { Column, EntityTableComponent } from '../entity-table/entity-table';
import { MultiPickComponent } from '../multi-pick/multi-pick';

type AssetDraft = AssetInput & { componentIds: string[]; flowIds: string[] };

export const ASSET_CATEGORIES: AssetCategory[] = [
  'business-data', 'identity-secrets', 'software-code', 'infrastructure', 'security-asset',
  'operational-asset', 'business-capability',
];

/** Assets: independent, linked to any number of components and flows (FR-063). */
@Component({
  selector: 'tm-assets-tab',
  imports: [FormsModule, EntityTableComponent, EditorDrawerComponent, MultiPickComponent],
  templateUrl: './assets-tab.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AssetsTabComponent {
  readonly store = inject(ThreatModelStore);
  readonly selectId = input<string | null>(null);
  readonly categories = ASSET_CATEGORIES;
  readonly cia: CiaLevel[] = ['low', 'medium', 'high'];
  readonly label = label;

  readonly componentOptions = computed(() => this.store.components().map(c => ({ id: c.id, label: c.name })));
  readonly flowOptions = computed(() => this.store.flows().map(f => ({
    id: f.id,
    label: targetName({ kind: 'flow', id: f.id }, this.store.components(), this.store.flows())
      + (f.label ? ` (${f.label})` : ''),
  })));

  readonly columns = computed<Column<Asset>[]>(() => [
    { key: 'name', label: 'Name', text: a => a.name },
    { key: 'category', label: 'Category', text: a => label(a.category) },
    { key: 'cia', label: 'C / I / A', text: a => [a.confidentiality, a.integrity, a.availability].map(x => label(x) || '—').join(' / ') },
    { key: 'pd', label: 'Personal data', text: a => (a.personalData ? 'Yes' : 'No') },
    { key: 'components', label: 'Components', text: a => `${a.componentIds.length}`, sort: a => a.componentIds.length },
    { key: 'flows', label: 'Flows', text: a => `${a.flowIds.length}`, sort: a => a.flowIds.length },
    { key: 'warnings', label: 'Warnings', text: a => a.warnings.map(w => `⚠ ${w.message}`).join('; ') },
  ]);

  readonly editor = new EditorState<Asset, AssetDraft>(
    (d, item) => this.persist(d, item),
    a => this.store.remove('assets', a.id),
  );

  constructor() {
    effect(() => {
      const a = this.store.assets().find(x => x.id === this.selectId());
      if (a) untracked(() => this.edit(a));
    });
  }

  add(): void {
    this.editor.open(null, {
      name: '', description: '', category: 'business-data', owner: '', confidentiality: null,
      integrity: null, availability: null, personalData: false, componentIds: [], flowIds: [],
    });
  }

  edit(a: Asset): void {
    this.editor.open(a, {
      name: a.name, description: a.description ?? '', category: a.category, owner: a.owner ?? '',
      confidentiality: a.confidentiality ?? null, integrity: a.integrity ?? null,
      availability: a.availability ?? null, personalData: a.personalData,
      componentIds: [...a.componentIds], flowIds: [...a.flowIds],
    });
  }

  /** Saves the asset, then the components and flows whose link to it changed. */
  private async persist(d: AssetDraft, item: Asset | null): Promise<void> {
    const { componentIds, flowIds, ...fields } = d;
    const saved = await this.store.save('assets', {
      ...fields, description: orNull(fields.description), owner: orNull(fields.owner),
    }, item ?? undefined);
    const id = saved.id;
    for (const c of this.store.components()) {
      const has = c.assetIds.includes(id), want = componentIds.includes(c.id);
      if (has === want) continue;
      const input = componentInput(c);
      input.assetIds = want ? [...input.assetIds, id] : input.assetIds.filter(x => x !== id);
      await this.store.save('components', input, c);
    }
    for (const f of this.store.flows()) {
      const has = f.assetIds.includes(id), want = flowIds.includes(f.id);
      if (has === want) continue;
      const input = flowInput(f);
      input.assetIds = want ? [...input.assetIds, id] : input.assetIds.filter(x => x !== id);
      await this.store.save('flows', input, f);
    }
  }
}
