import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { NodeShape } from '../../../models/state/model-editor.store';
import { ALL_SHAPES, shapeIcon } from '../../../models/state/node-shapes';
import {
  StyleChange, StyleSectionComponent, StyleTarget,
} from '../../../models/components/style-section/style-section';

/** What the panel knows about a single selected element. */
export interface SingleInfo {
  kind: 'node' | 'edge' | 'zone';
  /** The component's chosen shape; null is the type's default. */
  shape?: NodeShape | null;
  routing?: 'curved' | 'orthogonal';
  curved?: boolean;
}

/**
 * Properties of the diagram selection, as in the model editor: the style
 * section for one or several elements, a component's shape, a flow's routing.
 */
@Component({
  selector: 'tm-diagram-properties',
  imports: [StyleSectionComponent],
  templateUrl: './diagram-properties.html',
  styleUrl: './diagram-properties.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiagramPropertiesComponent {
  readonly heading = input<string>('');
  /** Inside the detail view's Properties tab: no frame, header or Edit button. */
  readonly bare = input(false);
  readonly targets = input.required<StyleTarget[]>();
  /** Set when exactly one element is selected. */
  readonly single = input<SingleInfo | null>(null);

  readonly styleChange = output<StyleChange>();
  readonly shapeChange = output<NodeShape | null>();
  readonly routingChange = output<'curved' | 'orthogonal'>();
  readonly straighten = output<void>();
  readonly resetStyle = output<void>();
  readonly edit = output<void>();
  readonly closed = output<void>();

  readonly shapes = ALL_SHAPES;
  readonly shapeIcon = shapeIcon;
}
