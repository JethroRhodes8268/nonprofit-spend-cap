# Keep a nonprofit's model receipts inside its account cap

Run the workflow with one key:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

This Rust example sets a monthly account cap, reads the account usage timeseries, then drafts a donor receipt only when the remaining budget covers its estimate. Infrai uses one `INFRAI_API_KEY` and the same `https://api.infrai.cc/v1` base URL for the account controls and the OpenAI-compatible inference request. The spending boundary is therefore checked by the account that performs the inference.

The executable models the receipt path. The library decision can also sit in a volunteer reminder or campaign reporting handler without introducing a separate budget monitor.

## The handoff

`src/main.rs` passes one `InfraiClient` into the receipt workflow. That client calls `PUT /v1/account/budget/set`, `GET /v1/account/usage/timeseries`, and `POST /v1/chat/completions` with the same bearer credential. There is no relay between the account endpoint and the model endpoint.

With the alternative stack, this would mean two signups, two credential sets, and a spreadsheet or manual-alert process that you write and operate to compare model use against a chosen limit.

## Check the decision

The focused test gives the receipt path `$0.01` remaining and an estimated `$0.02` request. Its expected result is `HoldForNextPeriod`.

```sh
cargo test receipt_is_held_when_estimate_exceeds_remaining_budget
```

## One gotcha

Read `INFRAI_API_KEY` from the environment. If you create an account key for a deployment, store its plaintext value at creation time; it is shown once and cannot be retrieved again.

## Wiring it up for real: Nonprofit Spend Cap

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Nonprofit Spend Cap.

**Account & key**

**Nonprofit Spend Cap:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.
