//! Threat lifecycle and treatment rules.
//! See docs/specification/09-threat-modeling.md#threat-lifecycle.

use super::{RiskTreatment, ThreatFields, ThreatModelError, ThreatStatus};

fn blank(s: &Option<String>) -> bool {
    s.as_deref().is_none_or(|v| v.trim().is_empty())
}

/// Checks a threat about to be saved and derives the status from the
/// treatment. `previous` is the stored threat on update, `None` on a manual
/// create. Rule candidates are created without passing through here.
pub fn check_threat(
    new: &mut ThreatFields,
    previous: Option<&ThreatFields>,
) -> Result<(), ThreatModelError> {
    if new.title.trim().is_empty() {
        return Err(ThreatModelError::new("title", "must not be empty"));
    }
    if new.targets.is_empty() {
        return Err(ThreatModelError::new(
            "targets",
            "a threat needs at least one component or flow",
        ));
    }
    let was = previous.map(|p| p.status);
    if new.status == ThreatStatus::Candidate && was != Some(ThreatStatus::Candidate) {
        return Err(ThreatModelError::new(
            "status",
            "only rules create candidates; a threat cannot return to candidate",
        ));
    }
    if new.treatment == RiskTreatment::Accept {
        if blank(&new.acceptance_rationale) {
            return Err(ThreatModelError::new(
                "acceptanceRationale",
                "required to accept a risk",
            ));
        }
        if blank(&new.accepted_by) {
            return Err(ThreatModelError::new(
                "acceptedBy",
                "required to accept a risk",
            ));
        }
        new.status = ThreatStatus::Accepted;
    } else if new.status == ThreatStatus::Accepted {
        let leaving_accept = previous.is_some_and(|p| {
            p.status == ThreatStatus::Accepted && p.treatment == RiskTreatment::Accept
        });
        if !leaving_accept {
            return Err(ThreatModelError::new(
                "status",
                "accepted requires the treatment accept",
            ));
        }
        new.status = ThreatStatus::Open;
    }
    if new.status == ThreatStatus::Dismissed && blank(&new.dismiss_reason) {
        return Err(ThreatModelError::new(
            "dismissReason",
            "required to dismiss a threat",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::testutil::threat_fields;
    use super::super::{TargetKind, ThreatTarget};
    use super::*;
    use uuid::Uuid;

    fn fields() -> ThreatFields {
        threat_fields(ThreatTarget {
            kind: TargetKind::Component,
            id: Uuid::nil(),
        })
    }

    #[test]
    fn manual_threat_needs_title_and_target_and_cannot_be_candidate() {
        let mut f = fields();
        assert!(check_threat(&mut f, None).is_ok());
        f.targets.clear();
        assert_eq!(check_threat(&mut f, None).unwrap_err().field, "targets");
        let mut f = fields();
        f.status = ThreatStatus::Candidate;
        assert!(check_threat(&mut f, None).is_err());
    }

    #[test]
    fn confirm_candidate_and_no_return() {
        let mut prev = fields();
        prev.status = ThreatStatus::Candidate;
        let mut f = prev.clone();
        f.status = ThreatStatus::Open;
        assert!(check_threat(&mut f, Some(&prev)).is_ok());
        let mut back = f.clone();
        back.status = ThreatStatus::Candidate;
        assert!(check_threat(&mut back, Some(&f)).is_err());
    }

    #[test]
    fn dismiss_needs_reason() {
        let mut f = fields();
        f.status = ThreatStatus::Dismissed;
        assert_eq!(
            check_threat(&mut f, None).unwrap_err().field,
            "dismissReason"
        );
        f.dismiss_reason = Some("not applicable".into());
        assert!(check_threat(&mut f, None).is_ok());
    }

    #[test]
    fn accept_needs_rationale_and_sets_status() {
        let mut f = fields();
        f.treatment = RiskTreatment::Accept;
        assert_eq!(
            check_threat(&mut f, None).unwrap_err().field,
            "acceptanceRationale"
        );
        f.acceptance_rationale = Some("low exposure".into());
        assert_eq!(check_threat(&mut f, None).unwrap_err().field, "acceptedBy");
        f.accepted_by = Some("J. Doe".into());
        check_threat(&mut f, None).unwrap();
        assert_eq!(f.status, ThreatStatus::Accepted);

        let prev = f.clone();
        let mut g = f.clone();
        g.treatment = RiskTreatment::Mitigate;
        check_threat(&mut g, Some(&prev)).unwrap();
        assert_eq!(g.status, ThreatStatus::Open);
    }

    #[test]
    fn accepted_without_accept_treatment_is_rejected() {
        let mut f = fields();
        f.status = ThreatStatus::Accepted;
        assert!(check_threat(&mut f, None).is_err());
    }
}
