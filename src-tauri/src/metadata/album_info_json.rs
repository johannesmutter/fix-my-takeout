use crate::db::queries;
use crate::metadata::apple_date;
use crate::fs::sanitize;
use log::info;
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlbumInfo {
    album_name: Option<String>,
    creation_date: Option<String>,
    allow_contributions: Option<bool>,
    is_public: Option<bool>,
    owner: Option<Person>,
    photos: Option<Vec<AlbumPhoto>>,
    participants: Option<Vec<Participant>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    full_name: Option<String>,
    appleid: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlbumPhoto {
    name: Option<String>,
    date_created: Option<String>,
    contributor: Option<Person>,
    comments: Option<Vec<PhotoComment>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhotoComment {
    is_like: Option<bool>,
    comment: Option<String>,
    timestamp: Option<String>,
    contributor: Option<Person>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Participant {
    participant: Option<Person>,
    sharing_date: Option<String>,
    sharing_status: Option<String>,
}

/// Stores a parsed album's photo date info for use by the cataloger.
#[derive(Debug, Clone)]
pub struct AlbumPhotoInfo {
    pub filename: String,
    pub date_created: Option<String>, // ISO 8601
    pub contributor_name: Option<String>,
    pub contributor_appleid: Option<String>,
}

/// Parse AlbumInfo.json and insert into DB.
/// Returns album_id and a list of photo info for the cataloger.
pub fn parse_and_load(
    json_path: &Path,
    source_zip: &str,
    conn: &Connection,
) -> Result<(i64, Vec<AlbumPhotoInfo>), String> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read {}: {}", json_path.display(), e))?;

    let album: AlbumInfo = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", json_path.display(), e))?;

    let album_name = album.album_name.as_deref().unwrap_or("Untitled Album");
    let folder_name = sanitize::sanitize_folder_name(album_name);

    let creation_date = album
        .creation_date
        .as_deref()
        .and_then(apple_date::apple_date_to_iso);

    let album_id = queries::insert_album(
        conn,
        album_name,
        "my_album",
        creation_date.as_deref(),
        album.owner.as_ref().and_then(|o| o.full_name.as_deref()),
        album.owner.as_ref().and_then(|o| o.appleid.as_deref()),
        album.is_public.unwrap_or(false),
        album.allow_contributions.unwrap_or(false),
        Some(source_zip),
        &folder_name,
    )
    .map_err(|e| format!("DB insert error: {}", e))?;

    // Insert participants
    if let Some(participants) = &album.participants {
        for p in participants {
            let person = p.participant.as_ref();
            let sharing_date = p
                .sharing_date
                .as_deref()
                .and_then(apple_date::apple_date_to_iso);
            let _ = queries::insert_album_participant(
                conn,
                album_id,
                person.and_then(|pp| pp.full_name.as_deref()),
                person.and_then(|pp| pp.appleid.as_deref()),
                sharing_date.as_deref(),
                p.sharing_status.as_deref(),
            );
        }
    }

    // Collect photo info for cataloger
    let mut photo_infos = Vec::new();
    if let Some(photos) = &album.photos {
        for photo in photos {
            if let Some(name) = &photo.name {
                let date_iso = photo
                    .date_created
                    .as_deref()
                    .and_then(apple_date::apple_date_to_iso);

                photo_infos.push(AlbumPhotoInfo {
                    filename: name.clone(),
                    date_created: date_iso,
                    contributor_name: photo
                        .contributor
                        .as_ref()
                        .and_then(|c| c.full_name.clone()),
                    contributor_appleid: photo
                        .contributor
                        .as_ref()
                        .and_then(|c| c.appleid.clone()),
                });

                // Insert comments/likes
                if let Some(comments) = &photo.comments {
                    for comment in comments {
                        let ts = comment
                            .timestamp
                            .as_deref()
                            .and_then(apple_date::apple_date_to_iso);
                        let _ = conn.execute(
                            "INSERT INTO photo_comments (img_name, album_id, is_like, comment_text, timestamp, author_name, author_appleid)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            rusqlite::params![
                                name,
                                album_id,
                                comment.is_like.unwrap_or(false) as i32,
                                comment.comment,
                                ts,
                                comment.contributor.as_ref().and_then(|c| c.full_name.as_deref()),
                                comment.contributor.as_ref().and_then(|c| c.appleid.as_deref()),
                            ],
                        );
                    }
                }
            }
        }
    }

    info!(
        "Loaded album '{}' with {} photos from {}",
        album_name,
        photo_infos.len(),
        json_path.display()
    );

    Ok((album_id, photo_infos))
}
