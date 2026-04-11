# Application architecture

How the web client, API server, and database interact in the Acme
Web App.

## Overview

The application uses a standard three-tier architecture: a
server-rendered HTMX frontend, an Axum API server, and a PostgreSQL
database. All state lives in the database; the API server is
stateless except for in-memory session caches.

## Request flow

1. Browser sends an HTMX request.
2. Axum middleware validates the session token.
3. Handler processes the request and queries PostgreSQL.
4. Handler returns an HTML fragment.
5. HTMX swaps the fragment into the DOM.
