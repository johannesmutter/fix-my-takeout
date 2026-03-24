use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ZipStatusRow {
    pub zip_name: String,
    pub zip_path: String,
    pub size_bytes: Option<i64>,
    pub status: String,
    pub safe_to_delete: bool,
    pub files_extracted: i64,
    pub files_organized: i64,
    pub files_total: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileRow {
    pub id: i64,
    pub source_zip: String,
    pub original_path: String,
    pub final_path: Option<String>,
    pub move_status: String,
    pub file_checksum: Option<String>,
    pub file_size: i64,
    pub date_taken: Option<String>,
    pub date_source: Option<String>,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub media_type: String,
    pub content_category: String,
    pub is_favourite: bool,
    pub is_hidden: bool,
    pub is_recently_deleted: bool,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryStats {
    pub total_files: i64,
    pub photos: i64,
    pub videos: i64,
    pub live_photos: i64,
    pub screenshots: i64,
    pub raw_images: i64,
    pub favourites: i64,
    pub hidden: i64,
    pub recently_deleted: i64,
    pub duplicates: i64,
    pub unknown_date: i64,
    pub albums_count: i64,
    pub files_per_year: Vec<(i32, i64)>,
}

pub fn upsert_zip_status(
    conn: &Connection,
    zip_name: &str,
    zip_path: &str,
    size_bytes: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO zip_status (zip_name, zip_path, size_bytes, status)
         VALUES (?1, ?2, ?3, 'pending')
         ON CONFLICT(zip_name) DO NOTHING",
        params![zip_name, zip_path, size_bytes],
    )?;
    Ok(())
}

pub fn update_zip_status(conn: &Connection, zip_name: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE zip_status SET status = ?1, started_at = CASE WHEN ?1 = 'extracting' THEN datetime('now') ELSE started_at END WHERE zip_name = ?2",
        params![status, zip_name],
    )?;
    Ok(())
}

pub fn mark_zip_done(conn: &Connection, zip_name: &str) -> Result<()> {
    conn.execute(
        "UPDATE zip_status SET status = 'done', safe_to_delete = 1, completed_at = datetime('now') WHERE zip_name = ?1",
        params![zip_name],
    )?;
    Ok(())
}

pub fn mark_zip_error(conn: &Connection, zip_name: &str, error: &str) -> Result<()> {
    conn.execute(
        "UPDATE zip_status SET status = 'error', error_message = ?1 WHERE zip_name = ?2",
        params![error, zip_name],
    )?;
    Ok(())
}

pub fn update_zip_file_counts(
    conn: &Connection,
    zip_name: &str,
    extracted: i64,
    organized: i64,
    total: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE zip_status SET files_extracted = ?1, files_organized = ?2, files_total = ?3 WHERE zip_name = ?4",
        params![extracted, organized, total, zip_name],
    )?;
    Ok(())
}

pub fn get_zip_statuses(conn: &Connection) -> Result<Vec<ZipStatusRow>> {
    let mut stmt = conn.prepare(
        "SELECT zip_name, zip_path, size_bytes, status, safe_to_delete,
                files_extracted, files_organized, files_total, error_message
         FROM zip_status ORDER BY zip_name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ZipStatusRow {
            zip_name: row.get(0)?,
            zip_path: row.get(1)?,
            size_bytes: row.get(2)?,
            status: row.get(3)?,
            safe_to_delete: row.get::<_, i64>(4)? != 0,
            files_extracted: row.get(5)?,
            files_organized: row.get(6)?,
            files_total: row.get(7)?,
            error_message: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn insert_photo_metadata(
    conn: &Connection,
    source_zip: &str,
    img_name: &str,
    file_checksum: Option<&str>,
    favorite: bool,
    hidden: bool,
    deleted: bool,
    original_creation_date: Option<&str>,
    parsed_date: Option<&str>,
    import_date_raw: Option<&str>,
    import_date_parsed: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO photo_metadata (source_zip, img_name, file_checksum, favorite, hidden, deleted,
         original_creation_date, parsed_date, import_date_raw, import_date_parsed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            source_zip,
            img_name,
            file_checksum,
            favorite as i32,
            hidden as i32,
            deleted as i32,
            original_creation_date,
            parsed_date,
            import_date_raw,
            import_date_parsed,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_shared_library(
    conn: &Connection,
    source_zip: &str,
    img_name: &str,
    contributed_by_me: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO shared_library_metadata (source_zip, img_name, contributed_by_me)
         VALUES (?1, ?2, ?3)",
        params![source_zip, img_name, contributed_by_me as i32],
    )?;
    Ok(())
}

pub fn insert_album(
    conn: &Connection,
    name: &str,
    source_type: &str,
    creation_date: Option<&str>,
    owner_name: Option<&str>,
    owner_appleid: Option<&str>,
    is_public: bool,
    allow_contributions: bool,
    source_zip: Option<&str>,
    folder_name: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO albums (name, source_type, creation_date, owner_name, owner_appleid,
         is_public, allow_contributions, source_zip, folder_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            name,
            source_type,
            creation_date,
            owner_name,
            owner_appleid,
            is_public as i32,
            allow_contributions as i32,
            source_zip,
            folder_name,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_album_participant(
    conn: &Connection,
    album_id: i64,
    full_name: Option<&str>,
    appleid: Option<&str>,
    sharing_date: Option<&str>,
    sharing_status: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO album_participants (album_id, full_name, appleid, sharing_date, sharing_status)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![album_id, full_name, appleid, sharing_date, sharing_status],
    )?;
    Ok(())
}

pub fn insert_file(
    conn: &Connection,
    source_zip: &str,
    source_type: &str,
    original_path: &str,
    file_size: i64,
    media_type: &str,
    content_category: &str,
    file_extension: Option<&str>,
    photo_meta_id: Option<i64>,
    file_checksum: Option<&str>,
    date_taken: Option<&str>,
    date_source: Option<&str>,
    year: Option<i32>,
    month: Option<i32>,
    is_favourite: bool,
    is_hidden: bool,
    is_recently_deleted: bool,
    contributed_by_me: Option<bool>,
    live_photo_id: Option<&str>,
    contributor_name: Option<&str>,
    contributor_appleid: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO files (source_zip, source_type, original_path, file_size, media_type, content_category,
         file_extension, photo_meta_id, file_checksum, date_taken, date_source, year, month,
         is_favourite, is_hidden, is_recently_deleted, contributed_by_me, live_photo_id,
         contributor_name, contributor_appleid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            source_zip,
            source_type,
            original_path,
            file_size,
            media_type,
            content_category,
            file_extension,
            photo_meta_id,
            file_checksum,
            date_taken,
            date_source,
            year,
            month,
            is_favourite as i32,
            is_hidden as i32,
            is_recently_deleted as i32,
            contributed_by_me.map(|b| b as i32),
            live_photo_id,
            contributor_name,
            contributor_appleid,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_file_move(
    conn: &Connection,
    file_id: i64,
    final_path: &str,
    move_status: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE files SET final_path = ?1, move_status = ?2 WHERE id = ?3",
        params![final_path, move_status, file_id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_file_pair(conn: &Connection, file_id: i64, pair_id: i64, pair_type: &str) -> Result<()> {
    match pair_type {
        "live_photo" => {
            conn.execute("UPDATE files SET live_photo_pair = ?1 WHERE id = ?2", params![pair_id, file_id])?;
        }
        "raw_jpeg" => {
            conn.execute("UPDATE files SET raw_jpeg_pair = ?1 WHERE id = ?2", params![pair_id, file_id])?;
        }
        "aae" => {
            conn.execute("UPDATE files SET aae_source = ?1 WHERE id = ?2", params![pair_id, file_id])?;
        }
        _ => {}
    }
    Ok(())
}

pub fn mark_duplicate(conn: &Connection, file_id: i64, duplicate_of: i64) -> Result<()> {
    conn.execute(
        "UPDATE files SET is_duplicate = 1, duplicate_of = ?1 WHERE id = ?2",
        params![duplicate_of, file_id],
    )?;
    Ok(())
}

pub fn insert_file_album(conn: &Connection, file_id: i64, album_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO file_albums (file_id, album_id) VALUES (?1, ?2)",
        params![file_id, album_id],
    )?;
    Ok(())
}

pub fn get_summary_stats(conn: &Connection) -> Result<SummaryStats> {
    let total_files: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE is_duplicate = 0", [], |r| r.get(0))?;
    let photos: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE media_type IN ('image', 'raw_image') AND is_duplicate = 0", [], |r| r.get(0))?;
    let videos: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE media_type = 'video' AND is_duplicate = 0", [], |r| r.get(0))?;
    let live_photos: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE media_type = 'live_photo_image' AND is_duplicate = 0", [], |r| r.get(0))?;
    let screenshots: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE media_type = 'screenshot' AND is_duplicate = 0", [], |r| r.get(0))?;
    let raw_images: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE media_type = 'raw_image' AND is_duplicate = 0", [], |r| r.get(0))?;
    let favourites: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE is_favourite = 1 AND is_duplicate = 0", [], |r| r.get(0))?;
    let hidden: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE is_hidden = 1 AND is_duplicate = 0", [], |r| r.get(0))?;
    let recently_deleted: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE is_recently_deleted = 1 AND is_duplicate = 0", [], |r| r.get(0))?;
    let duplicates: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE is_duplicate = 1", [], |r| r.get(0))?;
    let unknown_date: i64 = conn.query_row("SELECT COUNT(*) FROM files WHERE date_source = 'none' AND is_duplicate = 0", [], |r| r.get(0))?;
    let albums_count: i64 = conn.query_row("SELECT COUNT(*) FROM albums", [], |r| r.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT year, COUNT(*) FROM files WHERE year IS NOT NULL AND is_duplicate = 0 GROUP BY year ORDER BY year",
    )?;
    let files_per_year: Vec<(i32, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SummaryStats {
        total_files,
        photos,
        videos,
        live_photos,
        screenshots,
        raw_images,
        favourites,
        hidden,
        recently_deleted,
        duplicates,
        unknown_date,
        albums_count,
        files_per_year,
    })
}

pub fn update_zip_source_type(conn: &Connection, zip_name: &str, source_type: &str) -> Result<()> {
    conn.execute(
        "UPDATE zip_status SET source_type = ?1 WHERE zip_name = ?2",
        params![source_type, zip_name],
    )?;
    Ok(())
}

pub fn get_pending_files(conn: &Connection, zip_name: &str) -> Result<Vec<(i64, String, String, Option<i32>, Option<i32>, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, media_type, year, month, content_category, source_type
         FROM files WHERE source_zip = ?1 AND move_status = 'pending' AND is_duplicate = 0",
    )?;
    let rows = stmt.query_map(params![zip_name], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?;
    rows.collect()
}

pub fn lookup_photo_metadata(conn: &Connection, img_name: &str) -> Result<Option<(i64, Option<String>, Option<String>, bool, bool, bool, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_checksum, parsed_date, favorite, hidden, deleted, import_date_parsed
         FROM photo_metadata WHERE img_name = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![img_name], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get::<_, i64>(3)? != 0,
            row.get::<_, i64>(4)? != 0,
            row.get::<_, i64>(5)? != 0,
            row.get(6)?,
        ))
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn lookup_shared_library(conn: &Connection, img_name: &str) -> Result<Option<bool>> {
    let mut stmt = conn.prepare(
        "SELECT contributed_by_me FROM shared_library_metadata WHERE img_name = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![img_name], |row| {
        Ok(row.get::<_, i64>(0)? != 0)
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

#[allow(dead_code)]
pub fn get_files_for_dedup(conn: &Connection) -> Result<Vec<(i64, Option<String>, i64, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_checksum, file_size, import_date_parsed
         FROM files
         JOIN photo_metadata ON files.photo_meta_id = photo_metadata.id
         WHERE files.is_duplicate = 0
         ORDER BY file_checksum, import_date_parsed ASC",
    )?;

    // Fallback: also get files without photo_metadata
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
        ))
    })?;
    rows.collect()
}

pub fn get_files_for_symlinks(conn: &Connection) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_zip, original_path, final_path, move_status, file_checksum,
                file_size, date_taken, date_source, year, month, media_type, content_category,
                is_favourite, is_hidden, is_recently_deleted, is_duplicate
         FROM files WHERE move_status = 'done' AND is_duplicate = 0",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(FileRow {
            id: row.get(0)?,
            source_zip: row.get(1)?,
            original_path: row.get(2)?,
            final_path: row.get(3)?,
            move_status: row.get(4)?,
            file_checksum: row.get(5)?,
            file_size: row.get(6)?,
            date_taken: row.get(7)?,
            date_source: row.get(8)?,
            year: row.get(9)?,
            month: row.get(10)?,
            media_type: row.get(11)?,
            content_category: row.get(12)?,
            is_favourite: row.get::<_, i64>(13)? != 0,
            is_hidden: row.get::<_, i64>(14)? != 0,
            is_recently_deleted: row.get::<_, i64>(15)? != 0,
            is_duplicate: row.get::<_, i64>(16)? != 0,
        })
    })?;
    rows.collect()
}

pub fn get_album_files(conn: &Connection) -> Result<Vec<(String, i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.folder_name, f.id, f.final_path
         FROM file_albums fa
         JOIN albums a ON fa.album_id = a.id
         JOIN files f ON fa.file_id = f.id
         WHERE f.final_path IS NOT NULL AND f.is_duplicate = 0",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect()
}

/// Lightweight catalogue entry for the HTML viewer.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogueEntry {
    pub filename: String,
    pub path: String,
    pub media_type: String,
    pub date_taken: Option<String>,
    pub year: Option<i32>,
    pub file_size: i64,
    pub is_favourite: bool,
}

pub fn get_catalogue_entries(conn: &Connection) -> Result<Vec<CatalogueEntry>> {
    let mut stmt = conn.prepare(
        "SELECT
            REPLACE(final_path, RTRIM(final_path, REPLACE(final_path, '/', '')), '') as filename,
            final_path, media_type, date_taken, year, file_size, is_favourite
         FROM files
         WHERE move_status = 'done' AND is_duplicate = 0
         ORDER BY date_taken DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CatalogueEntry {
            filename: row.get(0)?,
            path: row.get(1)?,
            media_type: row.get(2)?,
            date_taken: row.get(3)?,
            year: row.get(4)?,
            file_size: row.get(5)?,
            is_favourite: row.get::<_, i64>(6)? != 0,
        })
    })?;
    rows.collect()
}

#[allow(dead_code)]
pub fn set_app_state(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_state (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_app_state(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}
