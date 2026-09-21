use nonprofit_spend_cap::nonprofit_service::{draft_receipt, InfraiClient, ReceiptDraft, ReceiptDecision, ServiceError};

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let client = InfraiClient::from_env()?;
    client.set_monthly_cap(25.0).await?;
    let usage = client.usage_timeseries().await?;
    println!("Usage timeseries: {usage}");

    let decision = draft_receipt(&client, ReceiptDraft {
        donor_name: "Mina Park".into(), gift_usd: 35.0, campaign: "library hours".into(),
    }, 2.00, 0.01).await?;
    match decision {
        ReceiptDecision::Generate => println!("Receipt drafted under the account cap."),
        ReceiptDecision::HoldForNextPeriod => println!("Receipt held before model spend would exceed the cap."),
    }
    Ok(())
}
