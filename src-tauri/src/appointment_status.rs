/// Defines the only status transitions allowed by the scheduling workflow.
/// Keeping this policy separate makes it reusable by the local backend and
/// the future network API service.
pub fn transition_allowed(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "scheduled" => matches!(to, "arrived" | "cancelled" | "no_show"),
        "arrived" => matches!(to, "in_progress" | "cancelled"),
        "in_progress" => matches!(to, "completed" | "cancelled"),
        "completed" | "cancelled" | "no_show" => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::transition_allowed;

    #[test]
    fn normal_visit_flow_is_allowed() {
        assert!(transition_allowed("scheduled", "arrived"));
        assert!(transition_allowed("arrived", "in_progress"));
        assert!(transition_allowed("in_progress", "completed"));
    }

    #[test]
    fn invalid_shortcuts_are_blocked() {
        assert!(!transition_allowed("scheduled", "completed"));
        assert!(!transition_allowed("scheduled", "in_progress"));
        assert!(!transition_allowed("arrived", "completed"));
    }

    #[test]
    fn terminal_states_cannot_be_reopened() {
        for state in ["completed", "cancelled", "no_show"] {
            assert!(!transition_allowed(state, "scheduled"));
            assert!(!transition_allowed(state, "arrived"));
        }
    }

    #[test]
    fn cancellation_and_no_show_rules_are_explicit() {
        assert!(transition_allowed("scheduled", "cancelled"));
        assert!(transition_allowed("scheduled", "no_show"));
        assert!(transition_allowed("arrived", "cancelled"));
        assert!(transition_allowed("in_progress", "cancelled"));
        assert!(!transition_allowed("arrived", "no_show"));
    }
}
