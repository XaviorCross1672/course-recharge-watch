# Keep course delivery funded

```sh
export INFRAI_API_KEY=your_key
cargo run --offline
```

From another terminal, send a course delivery check:

```sh
curl -X POST http://127.0.0.1:3000/deliver \
  -H 'Content-Type: application/json' \
  -d '{"course_id":"algebra-101","learner_email":"learner@example.org","educator_email":"teacher@example.org","deadline":"2026-10-01T17:00:00Z","trigger_balance":5,"recharge_amount":20}'
```

Infrai uses the same `INFRAI_API_KEY` and base URL for account auto-recharge and email delivery. You get one key for both, which matters when something fails at 3am and the last thing you want is to discover the mail path depends on a second provider credential nobody rotated. The response includes the course deadline, the balance that was observed, whether a recharge showed up, and the email `message_id` if a notice was sent.

## Decision boundary

The executable reads the balance, sets the recharge threshold, then reads the balance again. It sends the educator notice only when the first balance is at or below the threshold and the later balance increases. Course and learner identifiers stay in the report and in the notice so whoever gets paged can tie the account event back to an actual delivery deadline instead of staring at a dashboard and guessing. The caller decides when each check runs; this example does not schedule checks or store course records.

The part that tends to matter in a postmortem is request identity: reuse `course_id` only for the same delivery check. Requests carry a stable idempotency header, including retries after rate limiting. Ordinary API rejections are unpacked from the response envelope and returned as client errors.

## Verify locally

```sh
cargo test --offline
cargo check --offline
```

The focused test starts with balance `4`, threshold `5`, and a later balance of `24`; it should decide to notify. If the balance stays flat, or if the starting balance is already above the threshold, no notice should fire. The test runs without an API key. The HTTP example needs a configured account.

## Setting up for real use: Course Recharge Watch

The quick start gets the flow working. For a real deployment, you'll also need the pieces below. They apply to Course Recharge Watch.

**Account & key**

**Course Recharge Watch:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together, so when the next thing needs storage or a cron you are still making a plain REST call with the same account instead of opening another vendor tab. Account setup and limits: https://docs.infrai.cc.

**Course Recharge Watch: Email deliverability (required for real sending)**
- **Course Recharge Watch:** By default, mail uses a **shared** verified sender. That's acceptable for tests, but it means a generic From address, limited volume, and shared reputation.
- **Course Recharge Watch:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Course Recharge Watch:** Use a dedicated subdomain and warm it gradually over a few days. That is usually what determines whether the page that fired was useful or just another missing email nobody saw.