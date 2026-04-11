# Google OAuth

Integrates Google as a third-party authentication provider for
reduced signup friction.

## Flow overview

The user clicks "Sign in with Google" on the login page. The
browser redirects to Google's authorization URL. After the user
grants access, Google redirects back to our callback endpoint with
an authorization code. The callback handler exchanges the code for
tokens and creates or links the user account.

## Token handling

Google access tokens are not persisted. Refresh tokens are stored
encrypted. Sessions use our existing opaque token mechanism.
