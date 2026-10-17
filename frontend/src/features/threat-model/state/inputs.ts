import {
  ArchComponent, ArchComponentInput, Control, ControlInput, Flow, FlowInput, SecurityTest,
  SecurityTestInput, Threat, ThreatInput,
} from '../../../core/api/threat-model-api';

/** The input that replaces an item with itself; callers change what they need. */
export function componentInput(c: ArchComponent): ArchComponentInput {
  return {
    name: c.name, description: c.description ?? null, type: c.type, owner: c.owner ?? null,
    technology: c.technology ?? null, criticality: c.criticality ?? null,
    internetFacing: c.internetFacing, zoneId: c.zoneId ?? null,
    linkedComponentId: c.linkedComponentId ?? null, linkedFeatureId: c.linkedFeatureId ?? null,
    assetIds: [...c.assetIds],
  };
}

export function flowInput(f: Flow): FlowInput {
  return {
    sourceId: f.sourceId, targetId: f.targetId, relationship: f.relationship,
    label: f.label ?? null, protocol: f.protocol ?? null, bidirectional: f.bidirectional,
    authentication: f.authentication, encrypted: f.encrypted, assetIds: [...f.assetIds],
  };
}

export function threatInput(t: Threat): ThreatInput {
  return {
    title: t.title, category: t.category, scenario: t.scenario ?? null,
    attackerObjective: t.attackerObjective ?? null, preconditions: t.preconditions ?? null,
    consequences: t.consequences ?? null, targets: t.targets.map(x => ({ ...x })),
    assetIds: [...t.assetIds], controlIds: [...t.controlIds], status: t.status,
    dismissReason: t.dismissReason ?? null, inherent: t.inherent ? { ...t.inherent } : null,
    current: t.current ? { ...t.current } : null, treatment: t.treatment,
    riskOwner: t.riskOwner ?? null, dueDate: t.dueDate ?? null,
    acceptanceRationale: t.acceptanceRationale ?? null, acceptedBy: t.acceptedBy ?? null,
  };
}

export function controlInput(c: Control): ControlInput {
  return {
    name: c.name, description: c.description ?? null, type: c.type, category: c.category,
    status: c.status, owner: c.owner ?? null, acceptanceCriteria: c.acceptanceCriteria ?? null,
    componentIds: [...c.componentIds], threatIds: [...c.threatIds],
  };
}

export function testInput(t: SecurityTest): SecurityTestInput {
  return {
    title: t.title, objective: t.objective ?? null, preconditions: [...t.preconditions],
    steps: t.steps.map(s => ({ ...s })), type: t.type, status: t.status,
    threatIds: [...t.threatIds], controlIds: [...t.controlIds],
  };
}
