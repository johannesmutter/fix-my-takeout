use crate::db::queries;
use crate::metadata::apple_date;
use crate::fs::sanitize;
use log::info;
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SubscribedAlbum {
    album_name: Option<String>,
    files: Option<Vec<serde_json::Value>>,
    comments: Option<Vec<FileComment>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileComment {
    file_name: Option<String>,
    comments: Option<Vec<Comment>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comment {
    is_like: Option<bool>,
    comment: Option<String>,
    timestamp: Option<String>,
    contributor: Option<Contributor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contributor {
    full_name: Option<String>,
    appleid: Option<String>,
}

/// Parse Subscribed Albums.json and insert into DB.
pub fn parse_and_load(
    json_path: &Path,
    source_zip: &str,
    conn: &Connection,
) -> Result<usize, String> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read {}: {}", json_path.display(), e))?;

    let albums: Vec<SubscribedAlbum> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", json_path.display(), e))?;

    let mut count = 0;
    for album in &albums {
        let name = album.album_name.as_deref().unwrap_or("Untitled");
        let folder_name = sanitize::sanitize_folder_name(name);

        let album_id = queries::insert_album(
            conn,
            name,
            "subscribed",
            None,
            None,
            None,
            false,
            false,
            Some(source_zip),
            &folder_name,
        )
        .map_err(|e| format!("DB insert error: {}", e))?;

        // Insert comments/likes
        if let Some(file_comments) = &album.comments {
            for fc in file_comments {
                if let (Some(file_name), Some(comments)) = (&fc.file_name, &fc.comments) {
                    for c in comments {
                        let ts = c
                            .timestamp
                            .as_deref()
                            .and_then(apple_date::apple_date_to_iso);
                        let _ = conn.execute(
                            "INSERT INTO photo_comments (img_name, album_id, is_like, comment_text, timestamp, author_name, author_appleid)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            rusqlite::params![
                                file_name,
                                album_id,
                                c.is_like.unwrap_or(false) as i32,
                                c.comment,
                                ts,
                                c.contributor.as_ref().and_then(|co| co.full_name.as_deref()),
                                c.contributor.as_ref().and_then(|co| co.appleid.as_deref()),
                            ],
                        );
                    }
                }
            }
        }

        count += 1;
    }

    info!("Loaded {} subscribed albums from {}", count, json_path.display());
    Ok(count)
}
