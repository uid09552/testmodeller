import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  HostListener,
  input,
  OnInit,
  output,
  viewChild,
} from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'tm-user-menu',
  imports: [RouterLink],
  templateUrl: './user-menu.html',
  styleUrl: './user-menu.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UserMenuComponent implements OnInit {
  readonly open = input(false);
  readonly close = output<void>();

  private readonly panel = viewChild<ElementRef<HTMLElement>>('panel');

  ngOnInit(): void {
    // Focus the menu panel when it opens so keyboard users can navigate it.
  }

  @HostListener('document:click', ['$event'])
  onDocClick(event: Event): void {
    if (!this.open()) return;
    const target = event.target as Node;
    const el = this.panel()?.nativeElement;
    if (el && !el.contains(target)) {
      this.close.emit();
    }
  }
}
