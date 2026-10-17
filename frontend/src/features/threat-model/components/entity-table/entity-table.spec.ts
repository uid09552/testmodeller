import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { Column, EntityTableComponent } from './entity-table';

interface Row { id: string; name: string; n: number }

@Component({
  imports: [EntityTableComponent],
  template: `<tm-entity-table caption="Things" [rows]="rows()" [columns]="cols" [selectable]="true"
                              (open)="opened.push($event.id)" (checkedChange)="checked = $event" />`,
})
class HostComponent {
  rows = signal<Row[]>([
    { id: 'a', name: 'Banana', n: 2 }, { id: 'b', name: 'Apple', n: 10 }, { id: 'c', name: 'Cherry', n: 1 },
  ]);
  cols: Column<Row>[] = [
    { key: 'name', label: 'Name', text: r => r.name },
    { key: 'n', label: 'Count', text: r => `${r.n}`, sort: r => r.n },
  ];
  opened: string[] = [];
  checked: string[] = [];
}

describe('EntityTableComponent', () => {
  let host: HostComponent;
  let root: HTMLElement;
  let detect: () => void;

  beforeEach(() => {
    const f = TestBed.createComponent(HostComponent);
    f.detectChanges();
    host = f.componentInstance;
    root = f.nativeElement;
    detect = () => f.detectChanges();
  });

  const names = () => [...root.querySelectorAll('tbody tr')].map(r => r.querySelectorAll('td')[1]?.textContent?.trim());

  it('sorts by a column and flips the direction, with aria-sort', () => {
    const headers = root.querySelectorAll<HTMLButtonElement>('th .et__sort');
    headers[1].click();
    detect();
    expect(names()).toEqual(['Cherry', 'Banana', 'Apple']);
    expect(headers[1].closest('th')!.getAttribute('aria-sort')).toBe('ascending');
    headers[1].click();
    detect();
    expect(names()).toEqual(['Apple', 'Banana', 'Cherry']);
  });

  it('filters by text', () => {
    const input = root.querySelector<HTMLInputElement>('input[aria-label="Filter Things"]')!;
    input.value = 'an';
    input.dispatchEvent(new Event('input'));
    detect();
    expect(names()).toEqual(['Banana']);
    expect(root.textContent).toContain('1 of 3');
  });

  it('moves between rows with the arrows and opens one with Enter', () => {
    const rows = root.querySelectorAll<HTMLElement>('tbody tr');
    rows[0].focus();
    rows[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    expect(document.activeElement).toBe(rows[1]);
    rows[1].dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    expect(host.opened).toEqual(['b']);
  });

  it('reports checked rows for bulk actions', () => {
    root.querySelector<HTMLInputElement>('thead input[type="checkbox"]')!.click();
    expect(host.checked).toEqual(['a', 'b', 'c']);
  });
});
