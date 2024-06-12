use crate::colors;
use crate::commands::join::join_channel;
use crate::metadata::{Metadata, MetadataMap};
use crate::state::State;

use serde::{Deserialize, Serialize};
use songbird::input::{Compose, YoutubeDl};
use std::io::{BufRead, BufReader};
use std::{error::Error, ops::Sub, time::Duration};
use tokio::process::Command;
use tracing::debug;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
struct YouTubeTrack {
    url: String,
    title: String,
    channel: String,
    playlist: String,
    playlist_id: String,
    duration_string: String,
}

fn build_playlist_url(playlist_id: &str) -> String {
    format!("https://www.youtube.com/playlist?list={}", playlist_id)
}

async fn get_tracks(
    url: String,
) -> Result<Vec<YouTubeTrack>, Box<dyn Error + Send + Sync + 'static>> {
    let output = Command::new("yt-dlp")
        .args(vec![&url, "--flat-playlist", "-j"])
        .output()
        .await?;

    let reader = BufReader::new(output.stdout.as_slice());
    let tracks: Vec<YouTubeTrack> = reader
        .lines()
        .map_while(Result::ok)
        .flat_map(|line| serde_json::from_str(&line))
        .collect();
    tracing::debug!("tracks: {:?}", tracks);
    Ok(tracks)
}

pub(crate) async fn play(
    interaction: Box<InteractionCreate>,
    state: State,
    query: String,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    debug!(
        "play command in channel {:?} by {:?}",
        interaction.channel,
        interaction.author(),
    );

    let content = format!("Adding track(s) to the queue: {}", query);
    let embeds = vec![EmbedBuilder::new()
        .description(content)
        .color(colors::YELLOW)
        .build()];
    let interaction_response_data = InteractionResponseDataBuilder::new()
        .flags(MessageFlags::LOADING)
        .embeds(embeds)
        .build();
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
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

    let tracks = get_tracks(query).await?;

    if tracks.len() > 1 {
        let first_track = tracks.first().unwrap();
        let content = format!(
            "Adding playlist [{}]({})",
            first_track.playlist,
            build_playlist_url(&first_track.playlist_id)
        );
        let embeds = vec![EmbedBuilder::new()
            .description(content)
            .color(colors::BLURPLE)
            .build()];
        state
            .http
            .interaction(interaction.application_id)
            .update_response(&interaction.token)
            .embeds(Some(&embeds))?
            .await?;
    }

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        call.queue().resume()?;
    }

    let mut tracks_added = vec![];
    for track in &tracks {
        tracing::debug!("track: {:?}", track);
        let url = track.url.clone();
        let mut src = YoutubeDl::new(reqwest::Client::new(), url.to_string());
        if let Ok(metadata) = src.aux_metadata().await {
            debug!("metadata: {:?}", metadata);
            tracks_added.push((url.clone(), metadata.title.clone()));

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
                    url,
                });
            }
        }
    }
    let mut content = String::new();
    let num_tracks_added = tracks_added.len();
    match num_tracks_added {
        0 => {}
        1 => {
            let (title, url) = if let Some(track) = tracks_added.first() {
                let track = track.clone();
                (track.1.unwrap_or("Unknown".to_string()), track.0)
            } else {
                ("Unknown".to_string(), "".to_string())
            };
            content = format!("Added [{}]({}) to the queue", title, url);
        }
        _ => {
            let first_track = tracks.first().unwrap();
            content.push_str(&format!(
                "Adding playlist: [{}]({})\n",
                &first_track.playlist,
                build_playlist_url(&first_track.playlist_id)
            ));
            content.push_str(&format!(
                "Added {} tracks to the queue:\n",
                num_tracks_added
            ));
        }
    }

    let embeds = vec![EmbedBuilder::new()
        .description(content)
        .color(colors::BLURPLE)
        .build()];
    state
        .http
        .interaction(interaction.application_id)
        .update_response(&interaction.token)
        .embeds(Some(&embeds))?
        .await?;

    Ok(())
}
