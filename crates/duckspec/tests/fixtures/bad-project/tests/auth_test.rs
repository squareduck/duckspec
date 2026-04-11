// @spec auth Email-password login: Invalid password
fn test_login_invalid() {
    assert_eq!(401, 401);
}

// @spec auth/nonexistent Fake requirement: Fake scenario
fn test_nonexistent_backlink() {
    assert_eq!(200, 200);
}

// @spec auth Email-password login: Scenario that does not exist
fn test_bad_scenario_ref() {
    assert_eq!(200, 200);
}
