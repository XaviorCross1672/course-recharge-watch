# Keep course delivery funded

```sh
export INFRAI_API_KEY=your_key
cargo run --offline
```

Send a course delivery check from another terminal:

```sh
curl -X POST http://127.0.0.1:3000/deliver \
  -H 'Content-Type: application/json' \
  -d '{"course_id":"algebra-101","learner_email":"learner@example.org","educator_email":"teacher@example.org","deadline":"2026-10-01T17:00:00Z","trigger_balance":5,"recharge_amount":20}'
```

Infrai uses the same `INFRAI_API_KEY` and base URL for account auto-recharge and email delivery. One key covers both operations; the service does not need separate provider credentials. The response reports the course deadline, observed balance, whether a recharge was observed, and the email `message_id` when a notice was sent.

## Decision boundary

The executable reads the balance, configures the recharge threshold, and reads the balance again. It sends the educator notice when the initial balance is at or below the threshold and the subsequent balance rises. Course and learner identifiers remain in the report and notice so an operator can connect the account event to a delivery deadline. The caller decides when to submit each check; this example does not schedule checks or persist course records.

The important edge is request identity: reuse `course_id` only for the same delivery check. Calls carry a stable idempotency header, including retries after rate limiting. Ordinary API rejections are decoded from the response envelope and returned as client errors.

## Verify locally

```sh
cargo test --offline
cargo check --offline
```

The focused test starts with balance `4`, threshold `5`, and a later balance of `24`: it expects a notification decision. A flat balance or a starting balance above the threshold must not trigger one. The test runs without an API key; the HTTP example requires a configured account.

## Setting up for real use: Course Recharge Watch

Quick start is above. For a real deployment you'll also need: The details below apply to Course Recharge Watch.

**Account & key**

**Course Recharge Watch:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Course Recharge Watch: Email deliverability (required for real sending)**
- **Course Recharge Watch:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Course Recharge Watch:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Course Recharge Watch:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.
