import { AssetsTabComponent } from './assets-tab/assets-tab';
import { ComponentsTabComponent } from './components-tab/components-tab';
import { FlowsTabComponent } from './flows-tab/flows-tab';
import { ZonesTabComponent } from './zones-tab/zones-tab';
import { asset, component, flow, zone } from '../testing/fixtures';
import { byText, mount, setValue } from '../testing/setup';

describe('architecture tabs', () => {
  describe('Zones', () => {
    it('creates a zone with its trust level', async () => {
      const h = await mount(ZonesTabComponent);
      byText(h.root, 'button', 'Add zone').click();
      await h.settle();
      setValue(h.root.querySelector<HTMLInputElement>('input[name="name"]')!, 'Internet');
      await h.settle();
      const trust = h.root.querySelector<HTMLSelectElement>('select[name="trust"]')!;
      trust.selectedIndex = 0;
      trust.dispatchEvent(new Event('change'));
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      const [c] = h.api.writes('createItem');
      expect(c.args[0]).toBe('zones');
      expect(c.args[1]).toEqual(expect.objectContaining({ name: 'Internet', trustLevel: 0 }));
      expect(h.root.textContent).toContain('Internet');
    });

    it('says that deleting a zone leaves its components without one', async () => {
      const h = await mount(ZonesTabComponent, api => {
        api.data.zones = [zone('z', 'Internal', 3)];
        api.data.components = [component('a', 'A', { zoneId: 'z' }), component('b', 'B', { zoneId: 'z' })];
      });
      expect(h.fixture.componentInstance.deleteText(h.store.zones()[0])).toContain('Its 2 component(s) will have no zone');
      expect(h.root.textContent).toContain('2');
    });
  });

  describe('Components', () => {
    it('offers the types grouped by layer', async () => {
      const h = await mount(ComponentsTabComponent);
      byText(h.root, 'button', 'Add component').click();
      await h.settle();
      const groups = [...h.root.querySelectorAll('select[name="type"] optgroup')].map(g => g.getAttribute('label'));
      expect(groups).toEqual(['Client', 'Integration', 'Logic', 'Data', 'Identity and security', 'Platform', 'External']);
      expect(h.root.querySelector('select[name="type"] option[value="cicd-pipeline"]')?.textContent).toBe('CI/CD pipeline');
    });

    it('shows the server\'s reason when a name clashes', async () => {
      const h = await mount(ComponentsTabComponent);
      byText(h.root, 'button', 'Add component').click();
      await h.settle();
      setValue(h.root.querySelector<HTMLInputElement>('input[name="name"]')!, 'Web app');
      h.api.failNext = { status: 409, message: "a component named 'Web app' already exists" };
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.root.querySelector('[role="alert"]')?.textContent).toContain("a component named 'Web app' already exists");
    });

    it('links a test component of the project and an asset', async () => {
      const h = await mount(ComponentsTabComponent, api => { api.data.assets = [asset('r', 'Customer records')]; });
      byText(h.root, 'button', 'Add component').click();
      await h.settle();
      setValue(h.root.querySelector<HTMLInputElement>('input[name="name"]')!, 'Payments svc');
      const link = h.root.querySelector<HTMLSelectElement>('select[name="linkedComponent"]')!;
      link.selectedIndex = 1;
      link.dispatchEvent(new Event('change'));
      byText<HTMLLabelElement>(h.root, 'label', 'Customer records').querySelector('input')!.click();
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.api.writes('createItem')[0].args[1]).toEqual(expect.objectContaining({
        name: 'Payments svc', linkedComponentId: 'c1', assetIds: ['r'],
      }));
    });

    it('maps a component to a feature of the tree, optionally, and fills in its test component', async () => {
      const h = await mount(ComponentsTabComponent, api => {
        api.data.components = [component('a', 'Payments svc'), component('b', 'Unmapped svc')];
      });
      expect(h.root.querySelectorAll('tbody tr')[1].textContent).toContain('—');
      byText(h.root, 'tbody .link-btn', 'Payments svc').click();
      await h.settle();
      const sel = h.root.querySelector<HTMLSelectElement>('select[name="linkedFeature"]')!;
      expect([...sel.querySelectorAll('optgroup')].map(g => g.label)).toEqual(['Payments']);
      sel.selectedIndex = 1;
      sel.dispatchEvent(new Event('change'));
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.api.writes('updateItem')[0].args[3]).toEqual(expect.objectContaining({ linkedFeatureId: 'f1', linkedComponentId: 'c1' }));
      expect(h.root.textContent).toContain('Payments › Pay');
      // The other component stays unmapped: the mapping is optional.
      expect(h.root.querySelectorAll('tbody tr')[1].textContent).not.toContain('Payments › Pay');
    });

    it('names how many flows go with a deleted component', async () => {
      const h = await mount(ComponentsTabComponent, api => {
        api.data.components = [component('a', 'A'), component('b', 'B')];
        api.data.flows = [flow('f1', 'a', 'b'), flow('f2', 'b', 'a')];
      });
      expect(h.fixture.componentInstance.deleteText(h.store.components()[0])).toContain('2 data flow(s)');
    });

    it('shows warnings in the list', async () => {
      const h = await mount(ComponentsTabComponent, api => {
        api.data.components = [component('a', 'Edge', { warnings: [{ code: 'NO_ZONE', message: 'no exposure zone assigned' }] })];
      });
      expect(h.root.textContent).toContain('⚠ no exposure zone assigned');
    });
  });

  describe('Assets', () => {
    it('lists the components that share an asset and links new ones from the asset', async () => {
      const h = await mount(AssetsTabComponent, api => {
        api.data.components = [component('a', 'Customer API', { assetIds: ['r'] }), component('b', 'Reporting')];
        api.data.assets = [asset('r', 'Customer records', { componentIds: ['a'] })];
      });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      byText<HTMLLabelElement>(h.root, 'label', 'Reporting').querySelector('input')!.click();
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      const comp = h.api.writes('updateItem').find(c => c.args[0] === 'components')!;
      expect(comp.args[1]).toBe('b');
      expect((comp.args[3] as { assetIds: string[] }).assetIds).toEqual(['r']);
    });
  });

  describe('Data flows', () => {
    it('filters to flows that cross a trust boundary and shows the crossing', async () => {
      const h = await mount(FlowsTabComponent, api => {
        api.data.components = [component('a', 'Web app'), component('b', 'Customer API'), component('c', 'DB')];
        api.data.flows = [
          flow('f1', 'a', 'b', { crossing: { crosses: true, direction: 'into-more-trusted', fromTrust: 0, toTrust: 3 } }),
          flow('f2', 'b', 'c'),
        ];
      });
      expect(h.root.querySelectorAll('tbody tr')).toHaveLength(2);
      byText<HTMLLabelElement>(h.root, 'label', 'Crosses trust boundary').querySelector('input')!.click();
      await h.settle();
      const rows = h.root.querySelectorAll('tbody tr');
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain('Web app → Customer API');
      expect(rows[0].textContent).toContain('⇢ Crosses (0 → 3)');
    });

    it('saves a documented authentication method', async () => {
      const h = await mount(FlowsTabComponent, api => { api.data.components = [component('a', 'A'), component('b', 'B')]; });
      byText(h.root, 'button', 'Add data flow').click();
      await h.settle();
      setValue(h.root.querySelector<HTMLSelectElement>('select[name="authMode"]')!, 'method');
      await h.settle();
      setValue(h.root.querySelector<HTMLInputElement>('input[name="method"]')!, 'mTLS');
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.api.writes('createItem')[0].args[1]).toEqual(expect.objectContaining({
        sourceId: 'a', targetId: 'b', authentication: 'mTLS', encrypted: 'unknown',
      }));
    });
  });
});
