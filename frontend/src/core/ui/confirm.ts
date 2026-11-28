import { inject, Injectable } from '@angular/core';
import { MessageService } from '@siemens/ix-angular/standalone';

/**
 * Asks before a destructive action with an iX danger message (DESIGN.md →
 * Buttons), instead of the browser's `confirm()`. Resolves `true` only when
 * the user chooses the action; closing the dialog or Cancel is `false`.
 */
@Injectable({ providedIn: 'root' })
export class ConfirmService {
  private readonly messages = inject(MessageService);

  danger(title: string, message: string, action = 'Delete'): Promise<boolean> {
    return new Promise(resolve => {
      void this.messages
        .danger(title, message, action, 'Cancel', 'confirm', 'cancel')
        .then(event => event.once(result => resolve(result.payload === 'confirm')));
    });
  }
}
