import { TestBed } from '@angular/core/testing';
import { StyleChange, StyleSectionComponent, StyleTarget } from './style-section';

describe('StyleSectionComponent', () => {
  let fixture: ReturnType<typeof TestBed.createComponent<StyleSectionComponent>>;
  let root: HTMLElement;
  let changes: StyleChange[];

  async function render(targets: StyleTarget[]) {
    await TestBed.configureTestingModule({ imports: [StyleSectionComponent] }).compileComponents();
    fixture = TestBed.createComponent(StyleSectionComponent);
    fixture.componentRef.setInput('targets', targets);
    changes = [];
    fixture.componentInstance.changed.subscribe(c => changes.push(c));
    fixture.detectChanges();
    root = fixture.nativeElement;
  }

  const group = (name: string) => root.querySelector(`[aria-label="${name}"]`);
  const option = (name: string) => root.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;

  it('offers line, fill and text for a state, but no arrowhead', async () => {
    await render([{ kind: 'node' }]);
    expect(group('Line pattern')).not.toBeNull();
    expect(group('Line width')).not.toBeNull();
    expect(group('Line colour')).not.toBeNull();
    expect(group('Fill colour')).not.toBeNull();
    expect(group('Text size')).not.toBeNull();
    expect(group('Arrowhead')).toBeNull();
  });

  it('offers the arrowhead but no fill for a transition', async () => {
    await render([{ kind: 'edge' }]);
    expect(group('Arrowhead')).not.toBeNull();
    expect(group('Fill colour')).toBeNull();
  });

  it('offers only the shared keys for a state and a transition', async () => {
    await render([{ kind: 'node' }, { kind: 'edge' }]);
    expect(group('Line pattern')).not.toBeNull();
    expect(group('Text colour')).not.toBeNull();
    expect(option('Bold')).not.toBeNull();
    expect(group('Fill colour')).toBeNull();
    expect(group('Arrowhead')).toBeNull();
  });

  it('marks the current values, with solid and normal text as the defaults', async () => {
    await render([{ kind: 'edge', style: { dash: 'dashed', arrow: 'open' } }]);
    expect(option('Dashed').getAttribute('aria-checked')).toBe('true');
    expect(option('Solid').getAttribute('aria-checked')).toBe('false');
    expect(option('Open').getAttribute('aria-checked')).toBe('true');
    expect(option('Normal text').getAttribute('aria-checked')).toBe('true');
  });

  it('shows a mixed state where values differ, and choosing one sets it on all', async () => {
    await render([{ kind: 'node', style: { width: 1 } }, { kind: 'node', style: { width: 3 } }]);
    expect(option('Width 1 px').getAttribute('aria-checked')).toBe('mixed');
    expect(option('Width 3 px').getAttribute('aria-checked')).toBe('mixed');
    option('Width 2 px').click();
    expect(changes).toEqual([{ key: 'width', value: 2 }]);
  });

  it('emits null for the default and toggles flags', async () => {
    await render([{ kind: 'node', style: { dash: 'dotted', bold: true } }]);
    option('Solid').click();
    option('Bold').click();
    option('Italic').click();
    expect(changes).toEqual([
      { key: 'dash', value: null }, { key: 'bold', value: null }, { key: 'italic', value: true },
    ]);
    expect(option('Bold').getAttribute('aria-pressed')).toBe('true');
  });

  it('shows "Mixed" for differing colours and picks presets and custom colours', async () => {
    await render([{ kind: 'node', style: { fill: '#ef4444' } }, { kind: 'node' }]);
    const fill = group('Fill colour')!;
    expect(fill.textContent).toContain('Mixed');

    fill.querySelector<HTMLButtonElement>('[aria-label="Fill colour: Blue"]')!.click();
    const picker = fill.querySelector<HTMLInputElement>('input[type="color"]')!;
    picker.value = '#123456';
    picker.dispatchEvent(new Event('change'));
    fill.querySelector<HTMLButtonElement>('[aria-label="Fill colour: default"]')!.click();
    expect(changes).toEqual([
      { key: 'fill', value: '#4f6ef2' }, { key: 'fill', value: '#123456' }, { key: 'fill', value: null },
    ]);
  });

  it('shows a custom colour as the current value', async () => {
    await render([{ kind: 'node', style: { fill: '#123456' } }]);
    const fill = group('Fill colour')!;
    expect(fill.querySelector('.swatch--custom')!.classList).toContain('swatch--on');
    expect(fill.textContent).toContain('#123456');
    expect(fill.querySelector<HTMLInputElement>('input[type="color"]')!.value).toBe('#123456');
  });
});
