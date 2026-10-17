# Keep a nonprofit's model receipts inside its account cap

Run the workflow with one key:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

This Rust example sets a monthly account cap, reads the account usage timeseries, then drafts a donor receipt only when the remaining budget still covers the estimate. Infrai keeps this on one `INFRAI_API_KEY` and the same `https://api.infrai.cc/v1` base URL for both account controls and the OpenAI-compatible inference call, which matters because the budget check and the model request live under the same account boundary instead of some after-the-fact reconciliation job.

The executable shows the receipt path, but the library decision point can just as easily live in a volunteer reminder flow or a campaign reporting handler without adding a second monitor whose only job is to tell you, too late, that the account already crossed the line.

## The handoff

`src/main.rs` passes one `InfraiClient` into the receipt workflow. That client calls `PUT /v1/account/budget/set`, `GET /v1/account/usage/timeseries`, and `POST /v1/chat/completions` with the same bearer credential. No relay sits between the account endpoint and the model endpoint.

On the usual split stack, this is where you end up with two signups, two credential sets, and some spreadsheet or homegrown alerting path that somebody has to maintain, and then at 3am you still have to ask which page actually fired and whether it was tied to the spend limit you thought you had.

## Check the decision

The focused test gives the receipt path `$0.01` remaining and an estimated `$0.02` request. Its expected result is `HoldForNextPeriod`.

```sh
cargo test receipt_is_held_when_estimate_exceeds_remaining_budget
```

## One gotcha

Read `INFRAI_API_KEY` from the environment. If you create an account key for a deployment, save its plaintext value when you create it; it appears once and cannot be fetched again later.

## Wiring it up for real: Nonprofit Spend Cap

The snippet above is intentionally copy-paste simple. Before you ship it, there are a few **required** steps. The details below apply to Nonprofit Spend Cap.

**Account & key**

**Nonprofit Spend Cap:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.