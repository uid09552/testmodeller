//! Staleness of generated security tests.
//! See docs/specification/09-threat-modeling.md#stale-generated-tests.

use std::collections::HashMap;

use uuid::Uuid;

use super::{Control, SecurityTest, Threat, ThreatStatus};

/// Why a generated test is stale, or `None`. Manual tests are never stale.
pub fn stale_reason(
    test: &SecurityTest,
    threats: &HashMap<Uuid, &Threat>,
    controls: &HashMap<Uuid, &Control>,
) -> Option<String> {
    let source = test.source.as_ref()?;
    if !test
        .fields
        .threat_ids
        .iter()
        .any(|id| threats.contains_key(id))
    {
        return Some("no threat".to_owned());
    }
    let threat = threats.get(&source.threat_id);
    if threat.is_some_and(|t| t.fields.status == ThreatStatus::Dismissed) {
        return Some("threat dismissed".to_owned());
    }
    let control_id = source.control_id?;
    let still_linked = threat.is_some_and(|t| t.fields.control_ids.contains(&control_id));
    if still_linked && controls.contains_key(&control_id) {
        return None;
    }
    Some(match controls.get(&control_id) {
        Some(c) => format!("control unlinked: {}", c.display_id()),
        None => "control unlinked".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::super::*;
    use super::*;

    fn generated(threat: &Threat, control: Option<&Control>) -> SecurityTest {
        SecurityTest {
            audit: audit(),
            number: 1,
            fields: SecurityTestFields {
                title: "t".into(),
                objective: None,
                preconditions: vec![],
                steps: vec![],
                test_type: SecurityTestType::Other,
                status: SecurityTestStatus::Draft,
                threat_ids: vec![threat.audit.id],
                control_ids: control.map(|c| vec![c.audit.id]).unwrap_or_default(),
            },
            origin: SecurityTestOrigin::Generated,
            source: Some(TemplateSource {
                template_id: "AUTH-FLOW".into(),
                template_version: 1,
                threat_id: threat.audit.id,
                control_id: control.map(|c| c.audit.id),
            }),
            last_result: None,
        }
    }

    fn target() -> ThreatTarget {
        ThreatTarget {
            kind: TargetKind::Component,
            id: Uuid::nil(),
        }
    }

    #[test]
    fn fresh_test_is_not_stale_and_manual_never() {
        let mut t = threat(1, threat_fields(target()));
        let c = control(2, "Authn", ControlStatus::Planned);
        t.fields.control_ids = vec![c.audit.id];
        let test = generated(&t, Some(&c));
        let threats = HashMap::from([(t.audit.id, &t)]);
        let controls = HashMap::from([(c.audit.id, &c)]);
        assert_eq!(stale_reason(&test, &threats, &controls), None);
        let mut manual = test.clone();
        manual.source = None;
        assert_eq!(
            stale_reason(&manual, &HashMap::new(), &HashMap::new()),
            None
        );
    }

    #[test]
    fn dismissed_threat() {
        let mut t = threat(1, threat_fields(target()));
        t.fields.status = ThreatStatus::Dismissed;
        let test = generated(&t, None);
        let threats = HashMap::from([(t.audit.id, &t)]);
        assert_eq!(
            stale_reason(&test, &threats, &HashMap::new()).as_deref(),
            Some("threat dismissed")
        );
    }

    #[test]
    fn deleted_threat_leaves_no_threat() {
        let t = threat(1, threat_fields(target()));
        let mut test = generated(&t, None);
        test.fields.threat_ids.clear();
        assert_eq!(
            stale_reason(&test, &HashMap::new(), &HashMap::new()).as_deref(),
            Some("no threat")
        );
    }

    #[test]
    fn unlinked_or_deleted_control() {
        let mut t = threat(1, threat_fields(target()));
        let c = control(2, "Authn", ControlStatus::Planned);
        let test = generated(&t, Some(&c));
        let threats = HashMap::from([(t.audit.id, &t)]);
        let controls = HashMap::from([(c.audit.id, &c)]);
        assert_eq!(
            stale_reason(&test, &threats, &controls).as_deref(),
            Some("control unlinked: CTL-2")
        );
        t.fields.control_ids = vec![c.audit.id];
        let threats = HashMap::from([(t.audit.id, &t)]);
        assert_eq!(
            stale_reason(&test, &threats, &HashMap::new()).as_deref(),
            Some("control unlinked")
        );
    }
}
