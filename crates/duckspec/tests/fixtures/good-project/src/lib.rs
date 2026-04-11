pub fn login(_email: &str, _password: &str) -> u16 {
    200
}

pub fn logout(_token: &str) -> u16 {
    200
}

pub fn google_callback(_code: &str) -> u16 {
    302
}
