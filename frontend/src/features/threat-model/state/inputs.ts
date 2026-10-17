import {
  ArchComponent, ArchComponentInput, Asset, AssetInput, Control, ControlInput, Flow, FlowInput, SecurityTest,
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
    dataClassification: f.dataClassification ?? null, authorizationModel: f.authorizationModel ?? null,
    entryPoint: f.entryPoint ?? false,
  };
}

export function threatInput(t: Threat): ThreatInput {
  return {
    title: t.title, category: t.category, scenario: t.scenario ?? null,
    attackerObjective: t.attackerObjective ?? null, preconditions: t.preconditions ?? null,
    consequences: t.consequences ?? null, targets: t.targets.map(x => ({ ...x })),
    assetIds: [...t.assetIds], controlIds: [...t.controlIds], status: t.status,
    dismissReason: t.dismissReason ?? null, inherent: t.inherent ? { ...t.inherent } : null,
    current: t.current ? { ...t.current } : null, residual: t.residual ? { ...t.residual } : null,
    threatSource: t.threatSource ?? null, detection: t.detection ?? null,
    existingMitigations: t.existingMitigations ?? null, confidence: t.confidence ?? null,
    treatment: t.treatment,
    riskOwner: t.riskOwner ?? null, dueDate: t.dueDate ?? null,
    acceptanceRationale: t.acceptanceRationale ?? null, acceptedBy: t.acceptedBy ?? null,
    acceptanceExpires: t.acceptanceExpires ?? null,
  };
}

export function assetInput(a: Asset): AssetInput {
  return {
    name: a.name, description: a.description ?? null, category: a.category, owner: a.owner ?? null,
    confidentiality: a.confidentiality ?? null, integrity: a.integrity ?? null,
    availability: a.availability ?? null, personalData: a.personalData,
    classification: a.classification ?? null, dataLocation: a.dataLocation ?? null,
    retention: a.retention ?? null, encryptionAtRest: a.encryptionAtRest ?? 'unknown',
  };
}

export function controlInput(c: Control): ControlInput {
  return {
    name: c.name, description: c.description ?? null, type: c.type, category: c.category,
    status: c.status, owner: c.owner ?? null, acceptanceCriteria: c.acceptanceCriteria ?? null,
    reviewBy: c.reviewBy ?? null, componentIds: [...c.componentIds], threatIds: [...c.threatIds],
  };
}

export function testInput(t: SecurityTest): SecurityTestInput {
  return {
    title: t.title, objective: t.objective ?? null, preconditions: [...t.preconditions],
    steps: t.steps.map(s => ({ ...s })), type: t.type, status: t.status,
    threatIds: [...t.threatIds], controlIds: [...t.controlIds],
  };
}
