use crate::commands::join;
use crate::metadata::{Metadata, MetadataMap};
use crate::state::State;
use serde_json::Value;
use songbird::input::{Compose, YoutubeDl};
use std::io::{BufRead, BufReader};
use std::{error::Error, ops::Sub, time::Duration};
use tokio::process::Command;
use tracing::debug;
use twilight_model::channel::Message;
use url::Url;

pub(crate) async fn play(
    msg: Message,
    state: State,
    query: String,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "play command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );

    join(msg.clone(), state.clone()).await?;

    let guild_id = msg.guild_id.unwrap();

    // handle keyword queries
    let query = if Url::parse(&query).is_err() {
        format!("ytsearch:{query}")
    } else {
        query
    };

    // handle playlist links
    let urls = if query.contains("list=") {
        get_playlist_urls(query).await?
    } else {
        vec![query]
    };

    for url in urls {
        let mut src = YoutubeDl::new(reqwest::Client::new(), url.to_string());
        if let Ok(metadata) = src.aux_metadata().await {
            debug!("metadata: {:?}", metadata);

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
        } else {
            state
                .http
                .create_message(msg.channel_id)
                .content("Cannot find any results")?
                .await?;
        }
    }

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
