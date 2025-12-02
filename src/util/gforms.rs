use anyhow::anyhow;
use reqwest::StatusCode;
use serde::Serialize;

pub(crate) async fn submit_google_form(
    client: &reqwest::Client,
    form_id: &str,
    fields: &(impl Serialize + ?Sized),
) -> anyhow::Result<StatusCode> {
    let status = client
        .post(format!(
            "https://docs.google.com/forms/d/{form_id}/formResponse"
        ))
        .form(fields)
        .send()
        .await?
        .status();

    match status.is_success() {
        true => Ok(status),
        false => Err(anyhow!("Failed with status code {}", status)),
    }
}
