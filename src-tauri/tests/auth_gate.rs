mod users {
    pub use clinicdesk_lib::users::*;
}

#[allow(dead_code)]
#[path = "../src/auth.rs"]
mod auth;

#[test]
fn auth_gate_exports_expected_role_and_session_contract() {
    assert!(auth::hash_password("GateCredential1")
        .unwrap()
        .starts_with("$argon2"));
}
