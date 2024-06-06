use crate::commands::join::join_channel;
use crate::metadata::{Metadata, MetadataMap};
use crate::state::State;
use serde_json::Value;
use songbird::input::{Compose, YoutubeDl};
use std::io::{BufRead, BufReader};
use std::{error::Error, ops::Sub, time::Duration};
use tokio::process::Command;
use tracing::debug;
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;
use url::Url;

pub(crate) async fn play(
    interaction: Interaction,
    state: State,
    query: String,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    debug!(
        "play command in channel {:?} by {:?}",
        interaction.channel,
        interaction.author(),
    );

    let interaction_response_data = InteractionResponseDataBuilder::new()
        .content(format!("Adding track(s) to queue: {}", query))
        .build();
    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(interaction_response_data),
    };
    state
        .http
        .interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    let Some(user_id) = interaction.author_id() else {
        return Ok(());
    };
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    join_channel(state.clone(), guild_id, user_id).await?;

    // handle keyword queries
    let query = if Url::parse(&query).is_err() {
        format!("ytsearch:{query}")
    } else {
        query
    };

    debug!("query: {:?}", query);

    // handle playlist links
    let urls = if query.contains("list=") {
        get_playlist_urls(query).await?
    } else {
        vec![query]
    };

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        call.queue().resume()?;
    }

    let mut tracks_added = vec![];
    for url in urls {
        let mut src = YoutubeDl::new(reqwest::Client::new(), url.to_string());
        if let Ok(metadata) = src.aux_metadata().await {
            debug!("metadata: {:?}", metadata);
            tracks_added.push(metadata.title.clone());

            if let Some(call_lock) = state.songbird.get(guild_id) {
                let mut call = call_lock.lock().await;
                let handle = call.enqueue_with_preload(
                    src.into(),
                    metadata.duration.map(|duration| -> Duration {
                        if duration.as_secs() > 5 {
                            duration.sub(Duration::from_secs(5))
                        } else {
                            duration
                        }
                    }),
                );
                let mut x = handle.typemap().write().await;
                x.insert::<MetadataMap>(Metadata {
                    title: metadata.title,
                    duration: metadata.duration,
                });
            }
        }
    }
    let mut content = String::new();
    let num_tracks_added = tracks_added.len();
    match num_tracks_added {
        0 => {}
        1 => {
            content = format!(
                "Added \"{}\" to queue",
                tracks_added.first().unwrap().clone().unwrap()
            );
        }
        _ => {
            content = format!("Added {} tracks to queue:\n", num_tracks_added);
            for track in tracks_added.into_iter().take(num_tracks_added.min(3)) {
                content.push_str(&format!("  \"{}\"\n", track.unwrap()));
            }
        }
    }
    state
        .http
        .interaction(interaction.application_id)
        .update_response(&interaction.token)
        .content(Some(&content))
        .unwrap()
        .await?;

    Ok(())
}

async fn get_playlist_urls(
    url: String,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
    let output = Command::new("yt-dlp")
        .args(vec![&url, "--flat-playlist", "-j"])
        .output()
        .await?;

    let reader = BufReader::new(output.stdout.as_slice());
    let urls = reader
        .lines()
        .flatten()
        .map(|line| {
            let entry: Value = serde_json::from_str(&line).unwrap();
            entry
                .get("webpage_url")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    Ok(urls)
}
