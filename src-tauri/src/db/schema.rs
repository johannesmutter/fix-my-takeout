use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn open(output_dir: &Path) -> Result<Self> {
        let db_path = output_dir.join("catalog.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        let db = Database {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS zip_status (
                zip_name        TEXT PRIMARY KEY,
                zip_path        TEXT NOT NULL,
                size_bytes      INTEGER,
                status          TEXT NOT NULL DEFAULT 'pending',
                source_type     TEXT NOT NULL DEFAULT 'unknown',
                safe_to_delete  INTEGER NOT NULL DEFAULT 0,
                files_extracted INTEGER NOT NULL DEFAULT 0,
                files_organized INTEGER NOT NULL DEFAULT 0,
                files_total     INTEGER NOT NULL DEFAULT 0,
                csv_rows_loaded INTEGER NOT NULL DEFAULT 0,
                error_message   TEXT,
                started_at      TEXT,
                completed_at    TEXT
            );

            CREATE TABLE IF NOT EXISTS photo_metadata (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                source_zip              TEXT NOT NULL REFERENCES zip_status(zip_name),
                img_name                TEXT NOT NULL,
                file_checksum           TEXT,
                favorite                INTEGER NOT NULL DEFAULT 0,
                hidden                  INTEGER NOT NULL DEFAULT 0,
                deleted                 INTEGER NOT NULL DEFAULT 0,
                original_creation_date  TEXT,
                parsed_date             TEXT,
                import_date_raw         TEXT,
                import_date_parsed      TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_photo_meta_name ON photo_metadata(img_name);
            CREATE INDEX IF NOT EXISTS idx_photo_meta_checksum ON photo_metadata(file_checksum);

            CREATE TABLE IF NOT EXISTS shared_library_metadata (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                source_zip      TEXT NOT NULL REFERENCES zip_status(zip_name),
                img_name        TEXT NOT NULL,
                contributed_by_me INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_shared_lib_name ON shared_library_metadata(img_name);

            CREATE TABLE IF NOT EXISTS albums (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                source_type     TEXT NOT NULL,
                creation_date   TEXT,
                owner_name      TEXT,
                owner_appleid   TEXT,
                is_public       INTEGER NOT NULL DEFAULT 0,
                allow_contributions INTEGER NOT NULL DEFAULT 0,
                source_zip      TEXT,
                folder_name     TEXT
            );

            CREATE TABLE IF NOT EXISTS album_participants (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                album_id        INTEGER NOT NULL REFERENCES albums(id),
                full_name       TEXT,
                appleid         TEXT,
                sharing_date    TEXT,
                sharing_status  TEXT
            );

            CREATE TABLE IF NOT EXISTS files (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                source_zip      TEXT NOT NULL REFERENCES zip_status(zip_name),
                source_type     TEXT NOT NULL DEFAULT 'icloud',
                original_path   TEXT NOT NULL,
                final_path      TEXT,
                move_status     TEXT DEFAULT 'pending',

                photo_meta_id   INTEGER REFERENCES photo_metadata(id),
                file_checksum   TEXT,
                file_size       INTEGER NOT NULL,

                date_taken      TEXT,
                date_source     TEXT,
                year            INTEGER,
                month           INTEGER,

                media_type      TEXT NOT NULL,
                content_category TEXT NOT NULL DEFAULT 'photo',
                file_extension  TEXT,

                is_favourite    INTEGER NOT NULL DEFAULT 0,
                is_hidden       INTEGER NOT NULL DEFAULT 0,
                is_recently_deleted INTEGER NOT NULL DEFAULT 0,
                contributed_by_me INTEGER,

                live_photo_id   TEXT,
                live_photo_pair INTEGER REFERENCES files(id),
                raw_jpeg_pair   INTEGER REFERENCES files(id),
                aae_source      INTEGER REFERENCES files(id),

                is_duplicate    INTEGER NOT NULL DEFAULT 0,
                duplicate_of    INTEGER REFERENCES files(id),

                contributor_name  TEXT,
                contributor_appleid TEXT,

                processed_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_files_source ON files(source_zip);
            CREATE INDEX IF NOT EXISTS idx_files_checksum ON files(file_checksum);
            CREATE INDEX IF NOT EXISTS idx_files_date ON files(year, month);
            CREATE INDEX IF NOT EXISTS idx_files_live_photo ON files(live_photo_id);
            CREATE INDEX IF NOT EXISTS idx_files_move ON files(move_status);
            CREATE INDEX IF NOT EXISTS idx_files_ext ON files(file_extension);

            CREATE TABLE IF NOT EXISTS file_albums (
                file_id  INTEGER NOT NULL REFERENCES files(id),
                album_id INTEGER NOT NULL REFERENCES albums(id),
                PRIMARY KEY (file_id, album_id)
            );

            CREATE TABLE IF NOT EXISTS photo_comments (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id     INTEGER REFERENCES files(id),
                img_name    TEXT NOT NULL,
                album_id    INTEGER REFERENCES albums(id),
                is_like     INTEGER NOT NULL DEFAULT 0,
                comment_text TEXT,
                timestamp   TEXT,
                author_name TEXT,
                author_appleid TEXT
            );

            CREATE TABLE IF NOT EXISTS app_state (
                key   TEXT PRIMARY KEY,
                value TEXT
            );
            ",
        )?;
        Ok(())
    }
}
