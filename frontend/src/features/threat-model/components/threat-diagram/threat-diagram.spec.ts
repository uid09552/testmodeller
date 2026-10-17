import { ThreatDiagramComponent } from './threat-diagram';
import { component, flow, model, threat, zone } from '../../testing/fixtures';
import { mount } from '../../testing/setup';
import { FakeThreatModelApi } from '../../testing/fixtures';

/** Web app in Internet (0) → Customer API in Internal (3), plus a database and a user. */
function architecture(api: FakeThreatModelApi): void {
  api.model = model({
    layout: {
      v: 1,
      nodes: {
        web: { x: 40, y: 60, w: 150, h: 64 },
        api: { x: 340, y: 60, w: 150, h: 64 },
        db: { x: 340, y: 400, w: 150, h: 64 },
        user: { x: 40, y: 400, w: 150, h: 64 },
      },
      zones: {
        z0: { x: 10, y: 10, w: 220, h: 150 },
        z3: { x: 310, y: 10, w: 220, h: 150 },
        z9: { x: 600, y: 300, w: 300, h: 300 },
      },
      edges: {},
    },
  });
  api.data.zones = [zone('z0', 'Internet', 0), zone('z3', 'Internal', 3), zone('z9', 'Restricted', 4)];
  api.data.components = [
    component('web', 'Web app', { type: 'web-frontend', layer: 'client', zoneId: 'z0' }),
    component('api', 'Customer API', { type: 'api', layer: 'integration', zoneId: 'z3' }),
    component('db', 'Orders DB', { type: 'relational-database', layer: 'data' }),
    component('user', 'Customer', { type: 'human-user', layer: 'external' }),
  ];
  api.data.flows = [flow('f1', 'web', 'api', {
    label: 'HTTPS', crossing: { crosses: true, direction: 'into-more-trusted', fromTrust: 0, toTrust: 3 },
  })];
}

function pointer(type: string, x: number, y: number, init: MouseEventInit = {}): MouseEvent {
  return new MouseEvent(type, { clientX: x, clientY: y, bubbles: true, button: 0, ...init });
}

describe('ThreatDiagramComponent', () => {
  const node = (root: HTMLElement, name: string) =>
    root.querySelector<SVGGElement>(`.dg__node[aria-label^="${name},"]`)!;

  it('draws data stores as cylinders, external entities as rectangles and zones as labelled regions', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    expect(node(h.root, 'Orders DB').querySelector('path.dg__shape--data')).not.toBeNull();
    expect(node(h.root, 'Customer').querySelector('rect.dg__shape')).not.toBeNull();
    expect(node(h.root, 'Customer API').getAttribute('aria-label')).toBe('Customer API, REST/GraphQL API, Internal');
    const zones = [...h.root.querySelectorAll('.dg__zone-label')].map(z => z.textContent?.trim());
    expect(zones).toContain('Internal · trust 3');
  });

  it('marks a flow that crosses a trust boundary, not by colour alone', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const edge = h.root.querySelector('.dg__edge')!;
    expect(edge.getAttribute('aria-label')).toBe('Flow Web app → Customer API, HTTPS, crosses trust boundary');
    const marker = edge.querySelector('.dg__boundary')!;
    expect(marker.getAttribute('transform')).toBe('translate(230 92)');
  });

  it('shows open threats per element with the highest level and opens them', async () => {
    const h = await mount(ThreatDiagramComponent, api => {
      architecture(api);
      api.data.threats = [
        threat('t1', 1, { targets: [{ kind: 'component', id: 'api' }], currentLevel: 'high' }),
        threat('t2', 2, { targets: [{ kind: 'component', id: 'api' }], currentLevel: 'moderate' }),
      ];
    });
    h.root.querySelector<HTMLInputElement>('.dg__toggle input')!.click();
    await h.settle();
    const badge = h.root.querySelector('.dg__badge[aria-label^="2 open threats on Customer API"]')!;
    expect(badge.textContent).toContain('2 · ▆ High');
    badge.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(h.nav.go).toHaveBeenCalledWith('threats', { target: 'api' });
  });

  it('creates a flow from the pressed dot to the nearest dot of the target and opens its editor', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const svg = h.root.querySelector('svg')!;
    // Dots are top, right, bottom, left: press the bottom one of Customer API.
    node(h.root, 'Customer API').querySelectorAll('.dg__dot')[2].dispatchEvent(pointer('pointerdown', 415, 124));
    svg.dispatchEvent(pointer('pointermove', 415, 405));
    svg.dispatchEvent(pointer('pointerup', 415, 405));
    await h.settle();
    expect(h.api.writes('createItem')[0].args).toEqual(['flows', expect.objectContaining({
      sourceId: 'api', targetId: 'db', relationship: 'calls',
    })]);
    expect(h.root.querySelector('tm-editor-drawer')).not.toBeNull();
    const id = (h.api.data.flows.at(-1)!).id;
    expect(h.fixture.componentInstance.layout().edges[id]).toEqual(expect.objectContaining({ fromAnchor: 'bottom', toAnchor: 'top' }));
  });

  it('joins a flow without stored dots at the facing sides, not the centres', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const e = h.fixture.componentInstance.edges().find(x => x.f.id === 'f1')!;
    // Web app is left of Customer API: right side of one to the left side of the other.
    expect(e.g.src).toEqual({ x: 190, y: 92 });
    expect(e.g.tgt).toEqual({ x: 340, y: 92 });
  });

  it('re-attaches a flow end to another component', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const svg = h.root.querySelector('svg')!;
    h.root.querySelector('.dg__edge')!.dispatchEvent(pointer('pointerdown', 260, 92));
    await h.settle();
    h.root.querySelector('.dg__end--target')!.dispatchEvent(pointer('pointerdown', 340, 92));
    svg.dispatchEvent(pointer('pointermove', 345, 430));
    svg.dispatchEvent(pointer('pointerup', 345, 430));
    await h.settle();
    const upd = h.api.writes('updateItem').find(c => c.args[0] === 'flows')!;
    expect(upd.args[3]).toEqual(expect.objectContaining({ sourceId: 'web', targetId: 'db' }));
    expect(h.fixture.componentInstance.layout().edges['f1'].toAnchor).toBe('left');
  });

  it('styles every selected element and saves it with the layout', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    node(h.root, 'Web app').dispatchEvent(pointer('pointerdown', 50, 70));
    node(h.root, 'Customer API').dispatchEvent(pointer('pointerdown', 350, 70, { shiftKey: true }));
    h.root.querySelector('svg')!.dispatchEvent(pointer('pointerup', 350, 70));
    await h.settle();
    // Selecting does not open the panel; the right-click menu does.
    expect(h.root.querySelector('tm-diagram-properties')).toBeNull();
    node(h.root, 'Customer API').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 400, clientY: 90 }));
    await h.settle();
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Properties'))!.click();
    await h.settle();
    expect(h.root.querySelector('tm-diagram-properties')?.textContent).toContain('2 elements');
    h.fixture.componentInstance.applyStyle({ key: 'fill', value: '#fee2e2' });
    h.fixture.componentInstance.applyStyle({ key: 'dash', value: 'dashed' });
    await h.settle();
    const shape = node(h.root, 'Web app').querySelector<SVGElement>('.dg__shape')!;
    expect(shape.style.fill).toBe('rgb(254, 226, 226)');
    expect(shape.style.strokeDasharray).not.toBe('');
    expect(node(h.root, 'Customer API').querySelector<SVGElement>('.dg__shape')!.style.fill).toBe('rgb(254, 226, 226)');
    await new Promise(r => setTimeout(r, 700));
    const saved = h.api.writes('saveLayout').at(-1)!.args[2] as { nodes: Record<string, { style?: object }> };
    expect(saved.nodes['web'].style).toEqual({ fill: '#fee2e2', dash: 'dashed' });
  });

  it('draws a styled flow in its colour with a matching arrowhead', async () => {
    const h = await mount(ThreatDiagramComponent, api => {
      architecture(api);
      (api.model!.layout as { edges: object }).edges = { f1: { style: { stroke: '#dc2626', dash: 'dashed', arrow: 'open' } } };
    });
    const line = h.root.querySelector<SVGPathElement>('.dg__edge-line')!;
    expect(line.style.stroke).toBe('rgb(220, 38, 38)');
    expect(line.getAttribute('marker-end')).toBe('url(#arrow-open-dgdc2626)');
    expect(h.root.querySelector('marker#arrow-open-dgdc2626 polygon')).not.toBeNull();
  });

  it('offers the shapes, routing and actions in the right-click menus', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    node(h.root, 'Customer API').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 400, clientY: 90 }));
    await h.settle();
    const items = () => [...h.root.querySelectorAll('.ctx-menu [role="menuitem"]')].map(b => b.textContent?.trim());
    expect(items()).toEqual(expect.arrayContaining(['Make Hexagon', 'Move to Internet', 'Show Threats']));
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Make Hexagon'))!.click();
    await h.settle();
    expect(h.fixture.componentInstance.layout().nodes['api'].shape).toBe('hexagon');
    expect(h.root.querySelector('.ctx-menu')).toBeNull();

    h.root.querySelector('.dg__edge')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 260, clientY: 92 }));
    await h.settle();
    expect(items()).toContain('Right-Angle Routing');
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Right-Angle'))!.click();
    await h.settle();
    expect(h.fixture.componentInstance.layout().edges['f1'].routing).toBe('orthogonal');

    h.root.querySelector('.dg__edge')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 260, clientY: 92 }));
    await h.settle();
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Reverse'))!.click();
    await h.settle();
    expect(h.api.writes('updateItem').find(c => c.args[0] === 'flows')!.args[3])
      .toEqual(expect.objectContaining({ sourceId: 'api', targetId: 'web' }));
  });

  it('adds a component where the background was right-clicked, in that zone', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    h.root.querySelector('svg')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 700, clientY: 400 }));
    await h.settle();
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Here'))!.click();
    await h.settle();
    expect(h.api.writes('createItem')[0].args[0]).toBe('components');
    const upd = h.api.writes('updateItem').find(c => c.args[0] === 'components')!;
    expect((upd.args[3] as { zoneId: string }).zoneId).toBe('z9');
  });

  it('opens the detail view with Details and Properties tabs on a click', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const svg = h.root.querySelector('svg')!;
    node(h.root, 'Web app').dispatchEvent(pointer('pointerdown', 50, 70));
    svg.dispatchEvent(pointer('pointerup', 50, 70));
    await h.settle();
    const drawer = h.root.querySelector('tm-editor-drawer')!;
    expect([...drawer.querySelectorAll('[role="tab"]')].map(t => t.textContent?.trim())).toEqual(['Details', 'Properties']);
    expect(drawer.querySelector('input[name="name"]')).not.toBeNull();
    [...drawer.querySelectorAll<HTMLButtonElement>('[role="tab"]')][1].click();
    await h.settle();
    expect(drawer.querySelector('tm-diagram-properties')?.textContent).toContain('Line');
    // A drag does not open it.
    node(h.root, 'Orders DB').dispatchEvent(pointer('pointerdown', 350, 410));
    svg.dispatchEvent(pointer('pointermove', 450, 500, { altKey: true }));
    svg.dispatchEvent(pointer('pointerup', 450, 500));
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer')).toBeNull();
  });

  it('opens a flow or zone detail view on click and its Properties via the menu', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    h.root.querySelector('.dg__edge')!.dispatchEvent(pointer('pointerdown', 260, 92));
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer')?.textContent).toContain('Flow Web app → Customer API');
    h.root.querySelector('.dg__zone-head')!.dispatchEvent(pointer('pointerdown', 20, 20));
    h.root.querySelector('svg')!.dispatchEvent(pointer('pointerup', 20, 20));
    await h.settle();
    expect(h.root.querySelectorAll('tm-editor-drawer')).toHaveLength(1);
    expect(h.root.querySelector('tm-editor-drawer')?.textContent).toContain('Zone Internet');
    h.root.querySelector('.dg__zone')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 30, clientY: 30 }));
    await h.settle();
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Properties'))!.click();
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer [role="tab"][aria-selected="true"]')?.textContent).toContain('Properties');
  });

  it('closes the properties when the selection is cleared', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    node(h.root, 'Web app').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 60, clientY: 80 }));
    await h.settle();
    [...h.root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Properties'))!.click();
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer tm-diagram-properties')).not.toBeNull();
    h.root.querySelector('svg')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer')).toBeNull();
  });

  it('closes the menu with Escape', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    h.root.querySelector('svg')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 700, clientY: 400 }));
    await h.settle();
    h.root.querySelector('.ctx-menu')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await h.settle();
    expect(h.root.querySelector('.ctx-menu')).toBeNull();
  });

  it('moves a component into a zone region, which changes its zone, and saves the layout', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const svg = h.root.querySelector('svg')!;
    node(h.root, 'Orders DB').dispatchEvent(pointer('pointerdown', 350, 410));
    svg.dispatchEvent(pointer('pointermove', 650, 350, { altKey: true }));
    svg.dispatchEvent(pointer('pointerup', 650, 350));
    await h.settle();
    const upd = h.api.writes('updateItem').find(c => c.args[0] === 'components')!;
    expect(upd.args[1]).toBe('db');
    expect((upd.args[3] as { zoneId: string }).zoneId).toBe('z9');
    await new Promise(r => setTimeout(r, 700));
    const saved = h.api.writes('saveLayout').at(-1)!.args[2] as { nodes: Record<string, { x: number }> };
    expect(saved.nodes['db'].x).toBe(640);
  });

  it('places a component the layout does not know without overlapping others', async () => {
    const h = await mount(ThreatDiagramComponent, api => {
      architecture(api);
      api.data.components.push(component('new', 'Made by API'));
    });
    const l = h.fixture.componentInstance.layout();
    const r = l.nodes['new'];
    const overlaps = Object.entries(l.nodes).filter(([id]) => id !== 'new')
      .some(([, o]) => r.x < o.x + o.w && o.x < r.x + r.w && r.y < o.y + o.h && o.y < r.y + r.h);
    expect(overlaps).toBe(false);
  });

  it('zooms to fit with Shift+1 and opens a component with Enter', async () => {
    const h = await mount(ThreatDiagramComponent, architecture);
    const cmp = h.fixture.componentInstance;
    cmp.size.set({ w: 400, h: 300 });
    cmp.view.set({ zoom: 3, panX: 500, panY: 500 });
    h.root.querySelector('svg')!.dispatchEvent(new KeyboardEvent('keydown', { key: '!', shiftKey: true, bubbles: true }));
    expect(cmp.view().zoom).toBeLessThan(1);
    node(h.root, 'Web app').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await h.settle();
    expect(h.root.querySelector('tm-editor-drawer')?.textContent).toContain('Component Web app');
  });
});
