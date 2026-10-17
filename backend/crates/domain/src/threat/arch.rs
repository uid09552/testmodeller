//! The architecture aggregate, trust-boundary crossings and missing-information
//! warnings. See docs/specification/09-threat-modeling.md#computed-on-read.

use std::collections::HashMap;

use uuid::Uuid;

use super::{ArchComponent, ArchComponentType, ArchLayer, Asset, AuthState, Flow, TriState, Zone};

/// Everything describing what is being built, as read from storage.
#[derive(Debug, Clone, Default)]
pub struct Architecture {
    /// Exposure zones.
    pub zones: Vec<Zone>,
    /// Architecture components.
    pub components: Vec<ArchComponent>,
    /// Assets.
    pub assets: Vec<Asset>,
    /// Data flows.
    pub flows: Vec<Flow>,
}

impl Architecture {
    /// Zone by id.
    pub fn zone(&self, id: Uuid) -> Option<&Zone> {
        self.zones.iter().find(|z| z.audit.id == id)
    }

    /// Component by id.
    pub fn component(&self, id: Uuid) -> Option<&ArchComponent> {
        self.components.iter().find(|c| c.audit.id == id)
    }

    /// Asset by id.
    pub fn asset(&self, id: Uuid) -> Option<&Asset> {
        self.assets.iter().find(|a| a.audit.id == id)
    }

    /// Flow by id.
    pub fn flow(&self, id: Uuid) -> Option<&Flow> {
        self.flows.iter().find(|f| f.audit.id == id)
    }

    /// The zone a component runs in.
    pub fn zone_of(&self, component: &ArchComponent) -> Option<&Zone> {
        component.fields.zone_id.and_then(|z| self.zone(z))
    }

    /// Ids of the flows that carry an asset.
    pub fn flows_with_asset(&self, asset_id: Uuid) -> Vec<Uuid> {
        self.flows
            .iter()
            .filter(|f| f.fields.asset_ids.contains(&asset_id))
            .map(|f| f.audit.id)
            .collect()
    }

    /// Ids of the components that handle an asset.
    pub fn components_with_asset(&self, asset_id: Uuid) -> Vec<Uuid> {
        self.components
            .iter()
            .filter(|c| c.fields.asset_ids.contains(&asset_id))
            .map(|c| c.audit.id)
            .collect()
    }

    /// "Name (Zone, trust n)" or "Name (no zone)", for explanations.
    pub fn describe_component(&self, component: &ArchComponent) -> String {
        match self.zone_of(component) {
            Some(z) => format!(
                "{} ({}, trust {})",
                component.fields.name, z.fields.name, z.fields.trust_level
            ),
            None => format!("{} (no zone)", component.fields.name),
        }
    }

    /// "Source → Target" by component names.
    pub fn flow_name(&self, flow: &Flow) -> String {
        let name = |id| {
            self.component(id)
                .map_or("?".to_owned(), |c| c.fields.name.clone())
        };
        format!(
            "{} → {}",
            name(flow.fields.source_id),
            name(flow.fields.target_id)
        )
    }
}

/// Layer of a component type (09-threat-modeling.md, taxonomy).
pub fn layer(t: ArchComponentType) -> ArchLayer {
    use ArchComponentType as T;
    match t {
        T::WebFrontend | T::MobileApp | T::DesktopClient | T::BrowserExtension => ArchLayer::Client,
        T::ApiGateway | T::Api | T::MessageBroker | T::Webhook | T::ExternalConnector => {
            ArchLayer::Integration
        }
        T::Service | T::Worker | T::Scheduler | T::ServerlessFunction => ArchLayer::Logic,
        T::RelationalDatabase
        | T::ObjectStorage
        | T::Cache
        | T::SearchIndex
        | T::FileStore
        | T::Backup => ArchLayer::Data,
        T::IdentityProvider
        | T::AuthorizationService
        | T::SecretsManager
        | T::KeyManagement
        | T::AuditService => ArchLayer::IdentitySecurity,
        T::CloudService
        | T::ContainerPlatform
        | T::CicdPipeline
        | T::ThirdPartySaas
        | T::Library => ArchLayer::Platform,
        T::HumanUser | T::ExternalSystem => ArchLayer::External,
    }
}

/// Direction of a trust-boundary crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingDirection {
    /// Target is more trusted than source.
    IntoMoreTrusted,
    /// Target is less trusted than source.
    IntoLessTrusted,
}

impl CrossingDirection {
    /// Wire representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntoMoreTrusted => "into-more-trusted",
            Self::IntoLessTrusted => "into-less-trusted",
        }
    }
}

/// Whether and how a flow crosses a trust boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crossing {
    /// The flow crosses a boundary.
    pub crosses: bool,
    /// Direction, when it crosses.
    pub direction: Option<CrossingDirection>,
    /// Trust level of the source's zone.
    pub from_trust: Option<u8>,
    /// Trust level of the target's zone.
    pub to_trust: Option<u8>,
}

/// Computes the crossing of a flow. A component without a zone counts as
/// trust 0 for the direction.
pub fn crossing(arch: &Architecture, flow: &Flow) -> Crossing {
    let trust_of = |id: Uuid| {
        arch.component(id)
            .and_then(|c| arch.zone_of(c))
            .map(|z| (z.audit.id, z.fields.trust_level))
    };
    let from = trust_of(flow.fields.source_id);
    let to = trust_of(flow.fields.target_id);
    let crosses = match (from, to) {
        (Some((fz, ft)), Some((tz, tt))) => fz != tz && ft != tt,
        (Some(_), None) | (None, Some(_)) => true,
        (None, None) => false,
    };
    let from_trust = from.map(|(_, t)| t);
    let to_trust = to.map(|(_, t)| t);
    let direction = crosses.then(|| {
        if to_trust.unwrap_or(0) > from_trust.unwrap_or(0) {
            CrossingDirection::IntoMoreTrusted
        } else {
            CrossingDirection::IntoLessTrusted
        }
    });
    Crossing {
        crosses,
        direction,
        from_trust,
        to_trust,
    }
}

/// Missing-information warning on an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Stable code.
    pub code: &'static str,
    /// Message.
    pub message: &'static str,
}

const NO_ZONE: Warning = Warning {
    code: "NO_ZONE",
    message: "no exposure zone assigned",
};
const NO_OWNER: Warning = Warning {
    code: "NO_OWNER",
    message: "no owner",
};
const INTERNET_FACING_NO_FLOW: Warning = Warning {
    code: "INTERNET_FACING_NO_FLOW",
    message: "internet-facing but no data flow documented",
};
const CROSSING_AUTH: Warning = Warning {
    code: "CROSSING_AUTH_UNDOCUMENTED",
    message: "authentication not documented",
};
const CROSSING_ENCRYPTION: Warning = Warning {
    code: "CROSSING_ENCRYPTION_UNDOCUMENTED",
    message: "encryption in transit not documented",
};
const ASSET_UNLINKED: Warning = Warning {
    code: "ASSET_UNLINKED",
    message: "not linked to any component or flow",
};

/// Warnings of a component.
pub fn component_warnings(arch: &Architecture, component: &ArchComponent) -> Vec<Warning> {
    let mut out = Vec::new();
    if arch.zone_of(component).is_none() {
        out.push(NO_ZONE);
    }
    if component
        .fields
        .owner
        .as_deref()
        .is_none_or(|o| o.trim().is_empty())
    {
        out.push(NO_OWNER);
    }
    let id = component.audit.id;
    if component.fields.internet_facing
        && !arch
            .flows
            .iter()
            .any(|f| f.fields.source_id == id || f.fields.target_id == id)
    {
        out.push(INTERNET_FACING_NO_FLOW);
    }
    out
}

/// Warnings of a flow.
pub fn flow_warnings(arch: &Architecture, flow: &Flow) -> Vec<Warning> {
    let mut out = Vec::new();
    if crossing(arch, flow).crosses {
        if flow.fields.auth_state() == AuthState::Unknown {
            out.push(CROSSING_AUTH);
        }
        if flow.fields.encrypted == TriState::Unknown {
            out.push(CROSSING_ENCRYPTION);
        }
    }
    out
}

/// Warnings of an asset.
pub fn asset_warnings(arch: &Architecture, asset: &Asset) -> Vec<Warning> {
    let id = asset.audit.id;
    if arch.components_with_asset(id).is_empty() && arch.flows_with_asset(id).is_empty() {
        vec![ASSET_UNLINKED]
    } else {
        Vec::new()
    }
}

/// Total number of warnings in the architecture.
pub fn warning_count(arch: &Architecture) -> usize {
    arch.components
        .iter()
        .map(|c| component_warnings(arch, c).len())
        .sum::<usize>()
        + arch
            .flows
            .iter()
            .map(|f| flow_warnings(arch, f).len())
            .sum::<usize>()
        + arch
            .assets
            .iter()
            .map(|a| asset_warnings(arch, a).len())
            .sum::<usize>()
}

/// Number of flows that cross a trust boundary.
pub fn crossing_count(arch: &Architecture) -> usize {
    arch.flows
        .iter()
        .filter(|f| crossing(arch, f).crosses)
        .count()
}

/// Component id → number of flows starting or ending at it.
pub fn flow_degree(arch: &Architecture) -> HashMap<Uuid, usize> {
    let mut m = HashMap::new();
    for f in &arch.flows {
        *m.entry(f.fields.source_id).or_insert(0) += 1;
        *m.entry(f.fields.target_id).or_insert(0) += 1;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::*;

    fn two_zone_arch(t1: u8, t2: u8) -> (Architecture, Uuid) {
        let mut a = Architecture::default();
        let z1 = zone(&mut a, "Internet", t1);
        let z2 = zone(&mut a, "Internal", t2);
        let c1 = component(&mut a, "Web app", ArchComponentType::WebFrontend, Some(z1));
        let c2 = component(&mut a, "Customer API", ArchComponentType::Api, Some(z2));
        let f = flow(&mut a, c1, c2);
        (a, f)
    }

    #[test]
    fn internet_to_internal_crosses_into_more_trusted() {
        let (a, f) = two_zone_arch(0, 3);
        let c = crossing(&a, a.flow(f).unwrap());
        assert!(c.crosses);
        assert_eq!(c.direction, Some(CrossingDirection::IntoMoreTrusted));
        assert_eq!((c.from_trust, c.to_trust), (Some(0), Some(3)));
    }

    #[test]
    fn different_zones_same_trust_do_not_cross() {
        let (a, f) = two_zone_arch(3, 3);
        assert!(!crossing(&a, a.flow(f).unwrap()).crosses);
    }

    #[test]
    fn same_zone_does_not_cross() {
        let mut a = Architecture::default();
        let z = zone(&mut a, "Internal", 3);
        let c1 = component(&mut a, "A", ArchComponentType::Service, Some(z));
        let c2 = component(&mut a, "B", ArchComponentType::Service, Some(z));
        let f = flow(&mut a, c1, c2);
        assert!(!crossing(&a, a.flow(f).unwrap()).crosses);
    }

    #[test]
    fn exactly_one_zone_crosses_and_none_does_not() {
        let mut a = Architecture::default();
        let z = zone(&mut a, "Internal", 3);
        let c1 = component(&mut a, "A", ArchComponentType::Service, None);
        let c2 = component(&mut a, "B", ArchComponentType::Service, Some(z));
        let c3 = component(&mut a, "C", ArchComponentType::Service, None);
        let f1 = flow(&mut a, c1, c2);
        let f2 = flow(&mut a, c1, c3);
        let c = crossing(&a, a.flow(f1).unwrap());
        assert!(c.crosses);
        assert_eq!(c.direction, Some(CrossingDirection::IntoMoreTrusted));
        assert!(!crossing(&a, a.flow(f2).unwrap()).crosses);
    }

    #[test]
    fn zone_change_recomputes_crossing() {
        let (mut a, f) = two_zone_arch(0, 3);
        let z1 = a.zones[0].audit.id;
        a.components[1].fields.zone_id = Some(z1);
        assert!(!crossing(&a, a.flow(f).unwrap()).crosses);
    }

    #[test]
    fn crossing_flow_with_unknown_encryption_and_auth_warns() {
        let (mut a, f) = two_zone_arch(0, 3);
        let codes: Vec<_> = flow_warnings(&a, a.flow(f).unwrap())
            .iter()
            .map(|w| w.code)
            .collect();
        assert_eq!(
            codes,
            [
                "CROSSING_AUTH_UNDOCUMENTED",
                "CROSSING_ENCRYPTION_UNDOCUMENTED"
            ]
        );
        assert_eq!(
            flow_warnings(&a, a.flow(f).unwrap())[1].message,
            "encryption in transit not documented"
        );
        a.flows[0].fields.authentication = "none".into();
        a.flows[0].fields.encrypted = TriState::No;
        assert!(flow_warnings(&a, a.flow(f).unwrap()).is_empty());
    }

    #[test]
    fn component_and_asset_warnings() {
        let mut a = Architecture::default();
        let c = component(&mut a, "Edge", ArchComponentType::Api, None);
        a.components[0].fields.internet_facing = true;
        let codes: Vec<_> = component_warnings(&a, a.component(c).unwrap())
            .iter()
            .map(|w| w.code)
            .collect();
        assert_eq!(codes, ["NO_ZONE", "NO_OWNER", "INTERNET_FACING_NO_FLOW"]);
        let asset_id = asset(&mut a, "Records", None, None);
        assert_eq!(
            asset_warnings(&a, a.asset(asset_id).unwrap())[0].code,
            "ASSET_UNLINKED"
        );
        a.components[0].fields.asset_ids.push(asset_id);
        assert!(asset_warnings(&a, a.asset(asset_id).unwrap()).is_empty());
        assert_eq!(warning_count(&a), 3);
    }

    #[test]
    fn layers() {
        assert_eq!(
            layer(ArchComponentType::RelationalDatabase),
            ArchLayer::Data
        );
        assert_eq!(layer(ArchComponentType::HumanUser), ArchLayer::External);
        assert_eq!(layer(ArchComponentType::CicdPipeline), ArchLayer::Platform);
    }
}
