import {
  ChangeDetectionStrategy, Component, computed, effect, ElementRef, inject, signal, untracked,
  viewChildren,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router } from '@angular/router';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { AssetsTabComponent } from '../components/assets-tab/assets-tab';
import { ComponentsTabComponent } from '../components/components-tab/components-tab';
import { AiChatWindowComponent } from '../components/ai-chat-window/ai-chat-window';
import { BaselinesTabComponent } from '../components/baselines-tab/baselines-tab';
import { ReportTabComponent } from '../components/report-tab/report-tab';
import { ControlsTabComponent } from '../components/controls-tab/controls-tab';
import { FlowsTabComponent } from '../components/flows-tab/flows-tab';
import { OverviewTabComponent } from '../components/overview-tab/overview-tab';
import { RisksTabComponent } from '../components/risks-tab/risks-tab';
import { SecurityTestsTabComponent } from '../components/security-tests-tab/security-tests-tab';
import { ThreatDiagramComponent } from '../components/threat-diagram/threat-diagram';
import { ThreatsTabComponent } from '../components/threats-tab/threats-tab';
import { ZonesTabComponent } from '../components/zones-tab/zones-tab';
import { ThreatModelStore } from '../state/threat-model.store';
import { isTab, TabSlug, TABS } from '../state/threat-model-view';
import { AiTranscript } from '../state/ai-transcript';
import { WorkspaceNav } from '../state/workspace-nav';

/**
 * The Threat Modeling workspace of one project (06-ui.md): a project
 * dropdown, the tabs, and the active tab. The project and tab are in the URL.
 */
@Component({
  selector: 'tm-workspace-page',
  imports: [
    OverviewTabComponent, ThreatDiagramComponent, ComponentsTabComponent, FlowsTabComponent,
    ZonesTabComponent, AssetsTabComponent, ThreatsTabComponent, RisksTabComponent,
    ControlsTabComponent, SecurityTestsTabComponent, BaselinesTabComponent, ReportTabComponent, AiChatWindowComponent,
  ],
  providers: [ThreatModelStore, WorkspaceNav, AiTranscript],
  templateUrl: './workspace-page.html',
  styleUrl: './workspace-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class WorkspacePageComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly explorer = inject(ExplorerStore);
  private readonly nav = inject(WorkspaceNav);
  readonly store = inject(ThreatModelStore);

  readonly tabs = TABS;
  readonly projects = this.explorer.projects;
  private readonly params = toSignal(this.route.paramMap, { requireSync: true });
  readonly query = toSignal(this.route.queryParamMap, { requireSync: true });

  readonly projectId = computed(() => this.params().get('projectId'));
  readonly tab = computed<TabSlug>(() => {
    const t = this.params().get('tab');
    return isTab(t) ? t : 'overview';
  });
  readonly newName = signal('');
  /** Tab that has keyboard focus in the tab list. */
  readonly focusIndex = signal(0);
  private readonly tabButtons = viewChildren<ElementRef<HTMLButtonElement>>('tabBtn');

  constructor() {
    // No project in the URL: open the first one once the projects are known.
    effect(() => {
      const first = this.projects()[0];
      if (!this.projectId() && first) {
        untracked(() => void this.router.navigate(['/threat-modeling', first.id, 'overview'], { replaceUrl: true }));
      }
    });
    effect(() => {
      const id = this.projectId();
      this.nav.projectId = id;
      if (id) untracked(() => void this.store.load(id));
    });
    effect(() => {
      const i = TABS.findIndex(t => t.slug === this.tab());
      untracked(() => this.focusIndex.set(Math.max(0, i)));
    });
  }

  setProject(id: string): void {
    void this.router.navigate(['/threat-modeling', id, this.tab()]);
  }

  open(tab: TabSlug): void {
    this.nav.go(tab);
  }

  onTabKey(e: KeyboardEvent): void {
    const n = TABS.length;
    let i = this.focusIndex();
    if (e.key === 'ArrowRight') i = (i + 1) % n;
    else if (e.key === 'ArrowLeft') i = (i - 1 + n) % n;
    else if (e.key === 'Home') i = 0;
    else if (e.key === 'End') i = n - 1;
    else return;
    e.preventDefault();
    this.focusIndex.set(i);
    this.tabButtons()[i]?.nativeElement.focus();
  }

  async create(): Promise<void> {
    const project = this.projects().find(p => p.id === this.projectId());
    await this.store.create(this.newName().trim() || `${project?.name ?? 'Project'} threat model`);
  }
}
