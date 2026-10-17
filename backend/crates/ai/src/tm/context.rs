//! The context sent to the provider: one threat model, compact, capped.
//! Provider keys, tenant ids and user data are never part of it.

use std::collections::HashSet;

use serde_json::{json, Value};
use tm_domain::threat::arch::Architecture;
use tm_domain::threat::{Control, SecurityTest, Threat, ThreatModelFields, Thresholds};
use uuid::Uuid;

use super::payload::TmKind;

/// Largest context, in characters (about 24k tokens).
pub const MAX_CONTEXT_CHARS: usize = 96_000;
/// Longest description, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 400;
const MAX_ITEMS_WHEN_CUT: usize = 150;

/// What a request is about.
#[derive(Debug, Clone, Copy)]
pub struct TmContextInput<'a> {
    pub model: &'a ThreatModelFields,
    pub thresholds: &'a Thresholds,
    pub arch: &'a Architecture,
    pub threats: &'a [Threat],
    pub controls: &'a [Control],
    pub tests: &'a [SecurityTest],
}

/// The rendered context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedContext {
    /// JSON text, safe to place between `<context>` tags.
    pub text: String,
    /// The model was cut down to fit.
    pub truncated: bool,
}

fn cut(s: &str) -> String {
    let t: String = s.chars().take(MAX_DESCRIPTION_CHARS).collect();
    if s.chars().count() > MAX_DESCRIPTION_CHARS {
        format!("{t}…")
    } else {
        t
    }
}

fn opt(s: &Option<String>) -> Value {
    s.as_deref().map_or(Value::Null, |v| json!(cut(v)))
}

fn rating(r: Option<tm_domain::threat::Rating>) -> Value {
    r.map_or(
        Value::Null,
        |r| json!({"likelihood": r.likelihood, "impact": r.impact}),
    )
}

/// Ids that stay in a reduced context: the targets and their neighbours.
fn relevant(input: &TmContextInput, targets: &[Uuid]) -> HashSet<Uuid> {
    let mut keep: HashSet<Uuid> = targets.iter().copied().collect();
    for t in input
        .threats
        .iter()
        .filter(|t| targets.contains(&t.audit.id))
    {
        keep.extend(t.fields.targets.iter().map(|x| x.id));
        keep.extend(t.fields.control_ids.iter().copied());
    }
    for c in input
        .controls
        .iter()
        .filter(|c| targets.contains(&c.audit.id))
    {
        keep.extend(c.fields.threat_ids.iter().copied());
        keep.extend(c.fields.component_ids.iter().copied());
    }
    // One hop: flows touching a kept component, the components at their ends.
    let comps: HashSet<Uuid> = keep.clone();
    for f in &input.arch.flows {
        if comps.contains(&f.fields.source_id)
            || comps.contains(&f.fields.target_id)
            || keep.contains(&f.audit.id)
        {
            keep.insert(f.audit.id);
            keep.insert(f.fields.source_id);
            keep.insert(f.fields.target_id);
        }
    }
    keep
}

fn build(
    kind: TmKind,
    input: &TmContextInput,
    only: Option<&HashSet<Uuid>>,
    limit: Option<usize>,
) -> Value {
    let a = input.arch;
    let keep = |id: Uuid| only.is_none_or(|k| k.contains(&id));
    let take = |n: usize| limit.map_or(n, |l| l.min(n));
    let zone_name = |id: Option<Uuid>| {
        id.and_then(|z| a.zone(z))
            .map_or(Value::Null, |z| json!(z.fields.name))
    };
    let comp_name = |id: Uuid| {
        a.component(id)
            .map_or("?".to_owned(), |c| c.fields.name.clone())
    };

    let zones: Vec<Value> = a
        .zones
        .iter()
        .take(take(a.zones.len()))
        .map(|z| {
            json!({"name": z.fields.name, "kind": z.fields.kind, "trustLevel": z.fields.trust_level,
                   "description": opt(&z.fields.description)})
        })
        .collect();
    let components: Vec<Value> = a
        .components
        .iter()
        .filter(|c| keep(c.audit.id))
        .take(take(a.components.len()))
        .map(|c| {
            json!({"id": c.audit.id, "name": c.fields.name, "type": c.fields.component_type,
                   "zone": zone_name(c.fields.zone_id), "owner": opt(&c.fields.owner),
                   "criticality": c.fields.criticality, "internetFacing": c.fields.internet_facing,
                   "technology": opt(&c.fields.technology),
                   "description": opt(&c.fields.description),
                   "assets": c.fields.asset_ids.iter().filter_map(|i| a.asset(*i)).map(|x| &x.fields.name).collect::<Vec<_>>()})
        })
        .collect();
    let flows: Vec<Value> = a
        .flows
        .iter()
        .filter(|f| keep(f.audit.id))
        .take(take(a.flows.len()))
        .map(|f| {
            let c = tm_domain::threat::arch::crossing(a, f);
            json!({"id": f.audit.id, "source": comp_name(f.fields.source_id),
                   "target": comp_name(f.fields.target_id), "relationship": f.fields.relationship,
                   "protocol": opt(&f.fields.protocol), "authentication": f.fields.authentication,
                   "encrypted": f.fields.encrypted, "crossesTrustBoundary": c.crosses,
                   "assets": f.fields.asset_ids.iter().filter_map(|i| a.asset(*i)).map(|x| &x.fields.name).collect::<Vec<_>>()})
        })
        .collect();
    let assets: Vec<Value> = a
        .assets
        .iter()
        .take(take(a.assets.len()))
        .map(|x| {
            json!({"id": x.audit.id, "name": x.fields.name, "category": x.fields.category,
                   "confidentiality": x.fields.confidentiality, "integrity": x.fields.integrity,
                   "availability": x.fields.availability, "personalData": x.fields.personal_data,
                   "classification": x.fields.classification})
        })
        .collect();
    let threats: Vec<Value> = input
        .threats
        .iter()
        .filter(|t| keep(t.audit.id) || t.fields.targets.iter().any(|x| keep(x.id)))
        .take(take(input.threats.len()))
        .map(|t| {
            json!({"id": t.audit.id, "displayId": t.display_id(), "title": t.fields.title,
                   "category": t.fields.category, "status": t.fields.status,
                   "scenario": opt(&t.fields.scenario),
                   "targets": t.fields.targets,
                   "inherent": rating(t.fields.inherent), "current": rating(t.fields.current),
                   "residual": rating(t.fields.residual),
                   "controlIds": t.fields.control_ids})
        })
        .collect();
    let controls: Vec<Value> = input
        .controls
        .iter()
        .filter(|c| keep(c.audit.id) || c.fields.threat_ids.iter().any(|x| keep(*x)))
        .take(take(input.controls.len()))
        .map(|c| {
            json!({"id": c.audit.id, "displayId": c.display_id(), "name": c.fields.name,
                   "type": c.fields.control_type, "status": c.fields.status,
                   "threatIds": c.fields.threat_ids})
        })
        .collect();
    let mut v = json!({
        "threatModel": {
            "name": input.model.name,
            "scope": opt(&input.model.scope),
            "assumptions": opt(&input.model.assumptions),
            "exclusions": opt(&input.model.exclusions),
        },
        "zones": zones, "components": components, "flows": flows, "assets": assets,
        "threats": threats, "controls": controls,
    });
    if matches!(kind, TmKind::SecurityTests | TmKind::Review) {
        v["securityTests"] = json!(input
            .tests
            .iter()
            .take(take(input.tests.len()))
            .map(
                |t| json!({"displayId": t.display_id(), "title": t.fields.title,
                            "threatIds": t.fields.threat_ids, "controlIds": t.fields.control_ids})
            )
            .collect::<Vec<_>>());
    }
    v
}

/// Makes a JSON text safe between `<context>` tags: a `</` inside a string
/// cannot close the tag.
fn safe(text: String) -> String {
    text.replace("</", "<\\/")
}

/// Renders the context of a request. Over the cap it is reduced to the
/// targets and their neighbours, then to the first items of each list.
pub fn render(kind: TmKind, input: &TmContextInput, targets: &[Uuid]) -> RenderedContext {
    let full = safe(build(kind, input, None, None).to_string());
    if full.chars().count() <= MAX_CONTEXT_CHARS {
        return RenderedContext {
            text: full,
            truncated: false,
        };
    }
    let keep = relevant(input, targets);
    let reduced = safe(build(kind, input, (!keep.is_empty()).then_some(&keep), None).to_string());
    if reduced.chars().count() <= MAX_CONTEXT_CHARS {
        return RenderedContext {
            text: reduced,
            truncated: true,
        };
    }
    let mut limit = MAX_ITEMS_WHEN_CUT;
    loop {
        let text = safe(
            build(
                kind,
                input,
                (!keep.is_empty()).then_some(&keep),
                Some(limit),
            )
            .to_string(),
        );
        if text.chars().count() <= MAX_CONTEXT_CHARS || limit <= 5 {
            return RenderedContext {
                text,
                truncated: true,
            };
        }
        limit /= 2;
    }
}
