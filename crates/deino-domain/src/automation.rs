//! The closed sets automations (AUT-01) store: why a run started, where a
//! run stands and where each of its steps stands.

db_enum! {
    /// What started an automation's run. Stored in `automation_runs.trigger`.
    AutomationTrigger {
        /// The meeting finished processing (the `automations` pipeline stage).
        Pipeline = "pipeline",
        /// A tag its conditions test for was applied to the meeting.
        TagApplied = "tag_applied",
        /// The user ran it: from the meeting, the automation or a test run.
        Manual = "manual",
    }
}

db_enum! {
    /// Where an automation's run on a meeting stands. Stored in
    /// `automation_runs.status`.
    AutomationRunStatus {
        /// A tag armed it on a meeting that has not happened yet; it runs
        /// once the meeting is processed.
        Armed = "armed",
        /// Asked for; the `automations` stage has not run it yet.
        Pending = "pending",
        /// Every step ran (or was skipped) without failing.
        Done = "done",
        /// At least one step failed.
        Failed = "failed",
    }
}

impl AutomationRunStatus {
    /// True for runs the `automations` stage should pick up.
    pub const fn is_waiting(self) -> bool {
        matches!(self, Self::Armed | Self::Pending)
    }
}

db_enum! {
    /// Where one step of a run stands. Stored in `automation_run_steps.status`.
    AutomationStepStatus {
        /// Not run yet.
        Pending = "pending",
        /// Ran.
        Done = "done",
        /// Ran and failed; `error` says why.
        Failed = "failed",
        /// Not run: an earlier step failed and the automation stops on failure.
        Skipped = "skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn every_enum_serializes_as_its_schema_name() {
        assert_names_agree(AutomationTrigger::ALL, AutomationTrigger::as_str);
        assert_names_agree(AutomationRunStatus::ALL, AutomationRunStatus::as_str);
        assert_names_agree(AutomationStepStatus::ALL, AutomationStepStatus::as_str);
        assert_eq!(AutomationTrigger::TagApplied.as_str(), "tag_applied");
    }

    #[test]
    fn armed_and_pending_runs_wait_for_the_stage() {
        let waiting: Vec<AutomationRunStatus> = AutomationRunStatus::ALL
            .iter()
            .copied()
            .filter(|status| status.is_waiting())
            .collect();
        assert_eq!(
            waiting,
            [AutomationRunStatus::Armed, AutomationRunStatus::Pending]
        );
    }
}
