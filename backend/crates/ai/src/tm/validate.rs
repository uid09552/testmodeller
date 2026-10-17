//! Validation of provider output against the domain rules and the threat
//! model it was asked about. Invalid output is rejected, never repaired.

use std::collections::HashSet;

use tm_domain::threat::{Rating, TargetKind};
use uuid::Uuid;

use super::context::TmContextInput;
use super::payload::{ArchitecturePayload, TmPayload};
use crate::AiError;

fn invalid(m: impl Into<String>) -> AiError {
    AiError::InvalidOutput(m.into())
}

/// Names are the same when they match ignoring case and surrounding space.
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn blank(s: &str) -> bool {
    s.trim().is_empty()
}

struct Ids {
    threats: HashSet<Uuid>,
    controls: HashSet<Uuid>,
    components: HashSet<Uuid>,
    flows: HashSet<Uuid>,
    assets: HashSet<Uuid>,
    all: HashSet<Uuid>,
}

fn ids(input: &TmContextInput) -> Ids {
    let threats: HashSet<Uuid> = input.threats.iter().map(|t| t.audit.id).collect();
    let controls: HashSet<Uuid> = input.controls.iter().map(|t| t.audit.id).collect();
    let components: HashSet<Uuid> = input.arch.components.iter().map(|t| t.audit.id).collect();
    let flows: HashSet<Uuid> = input.arch.flows.iter().map(|t| t.audit.id).collect();
    let assets: HashSet<Uuid> = input.arch.assets.iter().map(|t| t.audit.id).collect();
    let mut all = HashSet::new();
    for s in [&threats, &controls, &components, &flows, &assets] {
        all.extend(s.iter().copied());
    }
    all.extend(input.arch.zones.iter().map(|z| z.audit.id));
    all.extend(input.tests.iter().map(|t| t.audit.id));
    Ids {
        threats,
        controls,
        components,
        flows,
        assets,
        all,
    }
}

fn all_in(set: &HashSet<Uuid>, xs: &[Uuid], what: &str) -> Result<(), AiError> {
    match xs.iter().find(|x| !set.contains(x)) {
        Some(x) => Err(invalid(format!(
            "{what}: {x} is not in this threat model; use ids from the context"
        ))),
        None => Ok(()),
    }
}

fn unique_new<'a>(
    names: impl Iterator<Item = &'a str>,
    existing: impl Iterator<Item = &'a str>,
    what: &str,
) -> Result<(), AiError> {
    let existing: Vec<&str> = existing.collect();
    let mut seen: Vec<&str> = Vec::new();
    for n in names {
        if blank(n) {
            return Err(invalid(format!("{what}: a name is empty")));
        }
        if existing.iter().any(|e| same_name(e, n)) {
            return Err(invalid(format!(
                "{what} \"{n}\" already exists; do not propose it again, refer to it by name"
            )));
        }
        if seen.iter().any(|e| same_name(e, n)) {
            return Err(invalid(format!("{what} \"{n}\" is proposed twice")));
        }
        seen.push(n);
    }
    Ok(())
}

fn architecture(p: &ArchitecturePayload, input: &TmContextInput) -> Result<(), AiError> {
    if p.zones.is_empty() && p.assets.is_empty() && p.components.is_empty() && p.flows.is_empty() {
        return Err(invalid("the architecture proposal is empty"));
    }
    let a = input.arch;
    unique_new(
        p.zones.iter().map(|z| z.name.as_str()),
        a.zones.iter().map(|z| z.fields.name.as_str()),
        "zone",
    )?;
    unique_new(
        p.assets.iter().map(|x| x.name.as_str()),
        a.assets.iter().map(|x| x.fields.name.as_str()),
        "asset",
    )?;
    unique_new(
        p.components.iter().map(|x| x.name.as_str()),
        a.components.iter().map(|x| x.fields.name.as_str()),
        "component",
    )?;
    if let Some(z) = p.zones.iter().find(|z| z.trust_level > 4) {
        return Err(invalid(format!(
            "zone \"{}\": trustLevel must be 0 to 4",
            z.name
        )));
    }
    let zone_known = |n: &str| {
        p.zones.iter().any(|z| same_name(&z.name, n))
            || a.zones.iter().any(|z| same_name(&z.fields.name, n))
    };
    let asset_known = |n: &str| {
        p.assets.iter().any(|x| same_name(&x.name, n))
            || a.assets.iter().any(|x| same_name(&x.fields.name, n))
    };
    let comp_known = |n: &str| {
        p.components.iter().any(|x| same_name(&x.name, n))
            || a.components.iter().any(|x| same_name(&x.fields.name, n))
    };
    for c in &p.components {
        if let Some(z) = &c.zone {
            if !zone_known(z) {
                return Err(invalid(format!(
                    "component \"{}\": unknown zone \"{z}\"",
                    c.name
                )));
            }
        }
        if let Some(x) = c.assets.iter().find(|x| !asset_known(x)) {
            return Err(invalid(format!(
                "component \"{}\": unknown asset \"{x}\"",
                c.name
            )));
        }
    }
    for f in &p.flows {
        for end in [&f.source, &f.target] {
            if !comp_known(end) {
                return Err(invalid(format!("flow: unknown component \"{end}\"")));
            }
        }
        if let Some(x) = f.assets.iter().find(|x| !asset_known(x)) {
            return Err(invalid(format!(
                "flow {} → {}: unknown asset \"{x}\"",
                f.source, f.target
            )));
        }
    }
    Ok(())
}

/// Checks a payload against the threat model.
pub fn validate(payload: &TmPayload, input: &TmContextInput) -> Result<(), AiError> {
    let known = ids(input);
    match payload {
        TmPayload::Architecture(p) => architecture(p, input),
        TmPayload::Threat(p) => {
            if blank(&p.title) {
                return Err(invalid("threat: title is empty"));
            }
            if p.targets.is_empty() {
                return Err(invalid(format!(
                    "threat \"{}\": at least one target (a component or flow id) is required",
                    p.title
                )));
            }
            for t in &p.targets {
                let set = match t.kind {
                    TargetKind::Component => &known.components,
                    TargetKind::Flow => &known.flows,
                };
                all_in(set, &[t.id], "threat target")?;
            }
            all_in(&known.assets, &p.asset_ids, "threat assetIds")
        }
        TmPayload::Rating(p) => {
            all_in(&known.threats, &[p.threat_id], "rating threatId")?;
            Rating::new(p.likelihood, p.impact, "rating")
                .map(|_| ())
                .map_err(|e| invalid(e.to_string()))
        }
        TmPayload::Control(p) => {
            if p.threat_ids.is_empty() {
                return Err(invalid("control: threatIds must name at least one threat"));
            }
            all_in(&known.threats, &p.threat_ids, "control threatIds")?;
            all_in(&known.components, &p.component_ids, "control componentIds")?;
            if let Some(link) = p.link_control_id {
                return all_in(&known.controls, &[link], "control linkControlId");
            }
            if p.name.as_deref().is_none_or(blank) {
                return Err(invalid("control: name is required"));
            }
            if p.control_type.is_none() || p.category.is_none() {
                return Err(invalid("control: type and category are required"));
            }
            Ok(())
        }
        TmPayload::Test(p) => {
            if blank(&p.title) {
                return Err(invalid("security test: title is empty"));
            }
            if p.threat_ids.is_empty() {
                return Err(invalid(
                    "security test: threatIds must name at least one threat",
                ));
            }
            if let Some(i) = p.steps.iter().position(|s| blank(&s.action)) {
                return Err(invalid(format!(
                    "security test: step {} has no action",
                    i + 1
                )));
            }
            all_in(&known.threats, &p.threat_ids, "security test threatIds")?;
            all_in(&known.controls, &p.control_ids, "security test controlIds")
        }
        TmPayload::Review(p) => {
            if p.findings.is_empty() || p.findings.iter().any(|f| blank(&f.text)) {
                return Err(invalid("review: findings must be non-empty texts"));
            }
            for f in &p.findings {
                all_in(&known.all, &f.entity_ids, "review entityIds")?;
            }
            Ok(())
        }
    }
}
