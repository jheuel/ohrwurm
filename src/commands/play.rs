use crate::colors;
use crate::commands::join::join_channel;
use crate::metadata::{Metadata, MetadataMap};
use crate::state::State;

use serde::{Deserialize, Serialize};
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::Track;
use std::io::{BufRead, BufReader};
use std::ops::Sub;
use std::{error::Error, time::Duration};
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
    url: Option<String>,
    original_url: Option<String>,
    title: String,
    channel: String,
    playlist: Option<String>,
    playlist_id: Option<String>,
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

    if tracks.is_empty() {
        if let Ok(stderr) = String::from_utf8(output.stderr) {
            if stderr.contains("This video is only available to Music Premium members") {
                return Err("This video is only available to Music Premium members".into());
            }
            if stderr.contains("YouTube said: The playlist does not exist.") {
                return Err("YouTube said: The playlist does not exist.".into());
            }
            return Err(stderr.into());
        }
        return Err("No tracks found".into());
    }
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

    let tracks = match get_tracks(query).await {
        Err(e) => {
            let content = format!("{}", e);
            let embeds = vec![EmbedBuilder::new()
                .description(content)
                .color(colors::RED)
                .build()];
            state
                .http
                .interaction(interaction.application_id)
                .update_response(&interaction.token)
                .embeds(Some(&embeds))?
                .await?;
            return Ok(());
        }
        Ok(tracks) => tracks,
    };

    if tracks.len() > 1 {
        let first_track = tracks.first().unwrap();
        let content = format!(
            "Adding playlist [{}]({})",
            first_track
                .playlist
                .clone()
                .unwrap_or("Unknown".to_string()),
            build_playlist_url(
                &first_track
                    .playlist_id
                    .clone()
                    .unwrap_or("Unknown".to_string())
            )
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
        let url = track.url.clone().or(track.original_url.clone()).ok_or("")?;
        let mut src = YoutubeDl::new(reqwest::Client::new(), url.clone());
        let src_copy = src.clone();
        let track: Track = src_copy.into();

        if let Ok(metadata) = src.aux_metadata().await {
            debug!("metadata: {:?}", metadata);
            tracks_added.push((url.clone(), metadata.title.clone()));

            if let Some(call_lock) = state.songbird.get(guild_id) {
                let mut call = call_lock.lock().await;
                let handle = call.enqueue_with_preload(
                    track,
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
                    src,
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
                &first_track
                    .playlist
                    .clone()
                    .unwrap_or("Unknown".to_string()),
                build_playlist_url(
                    &first_track
                        .playlist_id
                        .clone()
                        .unwrap_or("Unknown".to_string())
                )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_tracks() {
        let urls = [
            "https://www.youtube.com/playlist?list=PLFxxhcEeloYa1OlnWD6UgxlVQKJH5i_0p",
            "https://music.youtube.com/watch?v=RO75ZzqUOJw",
            "https://www.youtube.com/watch?v=qVHyl0P_P-M",
            "https://www.youtube.com/watch?v=34CZjsEI1yU",
        ];
        for url in urls.iter() {
            println!("url: {:?}", url);
            let tracks = get_tracks(url.to_string()).await.unwrap();
            assert!(!tracks.is_empty());
        }
    }

    #[tokio::test]
    async fn test_premium_tracks() {
        let urls = ["https://www.youtube.com/watch?v=QgMZRmxQ0Dc"];
        for url in urls.iter() {
            println!("url: {:?}", url);
            let tracks = get_tracks(url.to_string()).await;
            assert!(tracks.is_err());
            assert!(tracks
                .err()
                .unwrap()
                .to_string()
                .contains("This video is only available to Music Premium members"));
        }
    }

    #[tokio::test]
    async fn test_playlist_does_not_exist_tracks() {
        let urls = ["https://www.youtube.com/playlist?list=PLox0oG0uy8Lc1IaIfGyrvtuRItuEyJiyG"];
        for url in urls.iter() {
            println!("url: {:?}", url);
            let tracks = get_tracks(url.to_string()).await;
            assert!(tracks.is_err());
            assert!(tracks
                .err()
                .unwrap()
                .to_string()
                .contains("YouTube said: The playlist does not exist."));
        }
    }
}
