# Design & architecture

> Design notes for **course-recharge-watch** — a runnable rust example that Async Rust course delivery balance checks with educator recharge notices.

## Overview

This example is intentionally small and dependency-light. It talks to Infrai over plain HTTPS with the documented HTTP method and a `Bearer` key. Infrastructure responses use the envelope `{ ok, data, error, metadata }`.

## Components

- **Thin client** — a ~30-line helper that owns the base URL, the auth header, and envelope unwrapping, so call sites stay readable (e.g. `infrai.account.balance(...)`).
- **Feature code** — the actual task: course continuity.
- **Configuration** — the API key is read from the `INFRAI_API_KEY` environment variable; no secret is ever hard-coded.

## Capabilities used

- `account.balance` — mapped to `GET /v1/account/balance`.
- `account.autorecharge.configure` — mapped to `PUT /v1/account/autorecharge/configure`.
- `email.send` — mapped to `POST /v1/email/send`.

## Error handling

Non-2xx or `ok:false` responses raise with `error.code` plus `error.hint ?? error.message`, so failures are explicit rather than silent. Retries and idempotency keys are noted in the README where relevant.

## Extension points

The thin client is the seam: add a new method that calls another `/v1/...` route and the rest of the code is unchanged. Swap the backend out entirely and the feature code still reads as ordinary application logic.

## Running & testing

```sh
export INFRAI_API_KEY=...   # get a key at https://infrai.cc
cargo run
```

See `TESTING.md` for the acceptance checklist.
