// Log a Bits & Bytes Meetup using a context menu command

use crate::{
    AppError, AppVars, Context, attendance::roster_helpers::get_gsheets_token,
    util::modal::ModalInputTexts,
};
use anyhow::{Context as _, anyhow, bail};
use serde::Deserialize;
use serenity::all::{
    CacheHttp, CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal,
    EditInteractionResponse, InputTextStyle, ModalInteraction, ReactionType,
};

async fn submit_bnb_gform(
    data: &AppVars,
    fam_name: &str,
    msg_link: &str,
    meetup_type: &str,
) -> anyhow::Result<()> {
    let form_id = &data.env.bnb_form.id;
    let inputs = &data.env.bnb_form.input_ids;

    let status = reqwest::Client::new()
        .post(format!(
            "https://docs.google.com/forms/d/{form_id}/formResponse"
        ))
        .form(&vec![
            (&inputs.fam_name, fam_name),
            (&inputs.msg_link, msg_link),
            (&inputs.meetup_type, meetup_type),
        ])
        .send()
        .await?
        .status();

    if status.is_success() {
        Ok(())
    } else {
        dbg!(status);
        bail!("Google Form submission failed. Please check your inputs.")
    }
}

// TODO consolidate all google sheets helpers
#[derive(Debug, Deserialize)]
struct FlexibleSheetsResp {
    values: Vec<Vec<String>>,
}

async fn get_overview_range(data: &AppVars) -> anyhow::Result<FlexibleSheetsResp> {
    let access_token = get_gsheets_token(data).await?.access_token;
    let spreadsheet_id = &data.env.bnb_sheet.id;
    let spreadsheet_range = &data.env.bnb_sheet.lookup_range;

    let resp = reqwest::Client::new()
        .get(format!("https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{spreadsheet_range}"))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<FlexibleSheetsResp>()
        .await?;

    Ok(resp)
}

#[poise::command(context_menu_command = "Log B&B Meetup")]
pub(crate) async fn log_bnb_meetup_message(
    ctx: Context<'_>,
    message: serenity::all::Message,
) -> Result<(), AppError> {
    let submitter = &message.author.name;

    // get fam name from submitter
    let range = get_overview_range(ctx.data()).await?;
    let fam_name = range
        .values
        .into_iter()
        .find(|row| {
            if row.len() != 9 {
                return false;
            }

            let byte1 = &row[7];
            let byte2 = &row[8];

            submitter == byte1 || submitter == byte2
        })
        .map(|row| row[0].clone())
        .unwrap_or(String::from(""));

    let msg_input = CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Message Link", "message_link")
            .value(message.link())
            .required(true),
    );

    let fam_name_input = CreateActionRow::InputText(
        CreateInputText::new(
            InputTextStyle::Short,
            "Fam Name (case sensitive)",
            "fam_name",
        )
        .value(fam_name)
        .required(true),
    );

    let meetup_type_input = CreateActionRow::InputText(
        CreateInputText::new(
            InputTextStyle::Short,
            "Type (Hangout | Joint | Official B&B)",
            "meetup_type",
        )
        .value("Hangout")
        .required(true),
    );

    let modal = CreateModal::new("bnb_meetup_log_modal", "Log Bits & Bytes Meetup")
        .components(vec![msg_input, fam_name_input, meetup_type_input]);

    let reply = CreateInteractionResponse::Modal(modal);
    let Context::Application(ctx) = ctx else {
        bail!("unexpected context type")
    };

    ctx.interaction.create_response(ctx.http(), reply).await?;

    Ok(())
}

// TODO potentially make handlers deal with interactions...
pub(crate) async fn confirm_bnb_meetup_modal(
    ctx: serenity::prelude::Context,
    data: &'_ AppVars,
    ixn: ModalInteraction,
) -> Result<(), AppError> {
    let inputs = ModalInputTexts::new(&ixn);

    let message_link = inputs.get_required_value("message_link")?;
    let fam_name = inputs.get_required_value("fam_name")?;
    let meetup_type = inputs.get_required_value("meetup_type")?;

    ixn.defer_ephemeral(ctx.http()).await?;

    let message = message_link
        .split("/")
        .last()
        .ok_or(anyhow!("Invalid link"))
        .and_then(|id| {
            id.parse::<u64>()
                .context("unexpected non-numerical message ID")
        })
        .map(|id| ixn.channel_id.message(ctx.http(), id))
        .context("Cannnot find the original message")?
        .await?;

    // submit the B&B Google Form
    let response = match submit_bnb_gform(data, &fam_name, &message_link, &meetup_type).await {
        Ok(_) => "ok, logged",
        Err(why) => &why.to_string(),
    };

    ixn.edit_response(ctx.http(), EditInteractionResponse::new().content(response))
        .await?;

    let _ = message
        .react(ctx.http(), ReactionType::Unicode("👫".to_string()))
        .await;

    Ok(())
}
