//! Threat lifecycle and treatment rules.
//! See docs/specification/09-threat-modeling.md#threat-lifecycle.

use chrono::{Months, NaiveDate, Utc};

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
    check_threat_on(new, previous, Utc::now().date_naive())
}

/// Longest period a risk acceptance may last.
pub const MAX_ACCEPTANCE_MONTHS: u32 = 36;

fn check_expiry(
    new: &ThreatFields,
    previous: Option<&ThreatFields>,
    today: NaiveDate,
) -> Result<(), ThreatModelError> {
    let was_accepted = previous.filter(|p| p.treatment == RiskTreatment::Accept);
    match new.acceptance_expires {
        None => {
            // Acceptances recorded before expiry existed stay editable.
            if was_accepted.is_some_and(|p| p.acceptance_expires.is_none()) {
                return Ok(());
            }
            Err(ThreatModelError::new(
                "acceptanceExpires",
                "required to accept a risk",
            ))
        }
        Some(d) => {
            // An unchanged expiry is not re-validated, so editing other fields
            // of an expired acceptance is possible.
            if was_accepted.is_some_and(|p| p.acceptance_expires == Some(d)) {
                return Ok(());
            }
            let latest = today
                .checked_add_months(Months::new(MAX_ACCEPTANCE_MONTHS))
                .unwrap_or(today);
            if d <= today {
                Err(ThreatModelError::new(
                    "acceptanceExpires",
                    "must be after today",
                ))
            } else if d > latest {
                Err(ThreatModelError::new(
                    "acceptanceExpires",
                    "must be at most 3 years ahead",
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// [`check_threat`] with an explicit date.
pub fn check_threat_on(
    new: &mut ThreatFields,
    previous: Option<&ThreatFields>,
    today: NaiveDate,
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
        check_expiry(new, previous, today)?;
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
    use chrono::NaiveDate;
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
        assert_eq!(
            check_threat(&mut f, None).unwrap_err().field,
            "acceptanceExpires"
        );
        f.acceptance_expires = Some(chrono::Utc::now().date_naive() + chrono::Duration::days(90));
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

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn accepting(expires: Option<NaiveDate>) -> ThreatFields {
        let mut f = fields();
        f.treatment = RiskTreatment::Accept;
        f.acceptance_rationale = Some("r".into());
        f.accepted_by = Some("J. Doe".into());
        f.acceptance_expires = expires;
        f
    }

    #[test]
    fn acceptance_needs_an_expiry_in_range() {
        let today = day(2026, 10, 17);
        let mut ok = accepting(Some(day(2027, 10, 17)));
        assert!(check_threat_on(&mut ok, None, today).is_ok());
        assert_eq!(ok.status, ThreatStatus::Accepted);
        let mut none = accepting(None);
        assert_eq!(
            check_threat_on(&mut none, None, today).unwrap_err().field,
            "acceptanceExpires"
        );
        let mut past = accepting(Some(day(2026, 10, 16)));
        assert!(check_threat_on(&mut past, None, today).is_err());
        let mut today_itself = accepting(Some(today));
        assert!(check_threat_on(&mut today_itself, None, today).is_err());
        let mut far = accepting(Some(day(2029, 10, 18)));
        assert!(check_threat_on(&mut far, None, today).is_err());
        let mut edge = accepting(Some(day(2029, 10, 17)));
        assert!(check_threat_on(&mut edge, None, today).is_ok());
    }

    #[test]
    fn legacy_and_unchanged_acceptances_stay_editable() {
        let today = day(2026, 10, 17);
        let mut legacy_prev = accepting(None);
        legacy_prev.status = ThreatStatus::Accepted;
        let mut same = legacy_prev.clone();
        assert!(check_threat_on(&mut same, Some(&legacy_prev), today).is_ok());
        let mut expired_prev = accepting(Some(day(2026, 1, 1)));
        expired_prev.status = ThreatStatus::Accepted;
        let mut edit = expired_prev.clone();
        edit.title = "other".into();
        assert!(check_threat_on(&mut edit, Some(&expired_prev), today).is_ok());
        let mut renew = expired_prev.clone();
        renew.acceptance_expires = Some(day(2027, 4, 17));
        assert!(check_threat_on(&mut renew, Some(&expired_prev), today).is_ok());
        let mut stale = expired_prev.clone();
        stale.acceptance_expires = Some(day(2026, 2, 2));
        assert!(check_threat_on(&mut stale, Some(&expired_prev), today).is_err());
    }
}
