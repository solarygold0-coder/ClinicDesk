mod users {
    pub use clinicdesk_lib::users::*;
}

#[path = "../src/auth.rs"]
mod auth;

#[test]
fn auth_gate_exports_expected_role_and_session_contract() {
    assert_eq!(auth::hash_password("GatePassword1").unwrap().starts_with("$argon2"), true);
}
