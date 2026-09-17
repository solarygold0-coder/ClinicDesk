mod users {
    pub use clinicdesk_lib::users::*;
}

#[allow(dead_code)]
#[path = "../src/auth.rs"]
mod auth;

#[test]
fn exact_four_password_policy_keeps_argon2_hashing() {
    assert!(auth::hash_password("abc").is_err());
    assert!(auth::hash_password("abcde").is_err());
    assert!(auth::hash_password("A1!z").unwrap().starts_with("$argon2"));
}
