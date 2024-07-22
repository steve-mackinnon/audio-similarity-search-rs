use rusqlite::{Connection, Result};

use crate::file_utils;

pub struct MetadataDatabase {
    connection: Connection,
}

pub struct AudioFile {
    id: i64,
    path: String,
}

impl AudioFile {
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl MetadataDatabase {
    pub fn load_from_disk() -> Result<MetadataDatabase, String> {
        let file_path = file_utils::metadata_db_path()?;
        let connection = Connection::open(&file_path)
            .map_err(|e| format!("Failed to create database: {}", e))?;

        Ok(MetadataDatabase { connection })
    }

    /// Creates necessary db tables and inserts an entry for analysis_root_dir.
    /// Returns the analysis root dir ID on success.
    pub fn initialize(&self, analysis_root_dir: &str) -> Result<i64, String> {
        self.connection
            .execute(
                "CREATE TABLE IF NOT EXISTS analysis_root_dirs (
                    id INTEGER PRIMARY KEY,
                    dir_path TEXT NOT NULL
                )",
                (),
            )
            .map_err(|e| e.to_string())?;

        self.connection
            .execute(
                "CREATE TABLE IF NOT EXISTS samples (
                    id INTEGER PRIMARY KEY,
                    analysis_root_dir_id INTEGER,
                    file_path TEXT NOT NULL,
                    FOREIGN KEY(analysis_root_dir_id) REFERENCES analysis_root_dirs(id)
                )",
                (),
            )
            .map_err(|e| format!("Failed to create db table: {}", e))?;

        self.connection
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_file_path ON samples (file_path)",
                (),
            )
            .map_err(|e| format!("Failed to create db index: {}", e))?;

        let id = self.get_id_for_analysis_dir(analysis_root_dir)?;
        Ok(id)
    }

    fn get_id_for_analysis_dir(&self, analysis_root_dir: &str) -> Result<i64, String> {
        let root_dir_query = "SELECT id from analysis_root_dirs WHERE dir_path=?1";
        let mut root_dir_stmt = self
            .connection
            .prepare(root_dir_query)
            .map_err(|e| e.to_string())?;

        let mut rows = root_dir_stmt
            .query(rusqlite::params![analysis_root_dir])
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: i64 = row.get(0).map_err(|e| e.to_string())?;
            Ok(id)
        } else {
            self.connection
                .execute(
                    "INSERT INTO analysis_root_dirs (dir_path) VALUES (?1)",
                    [&analysis_root_dir],
                )
                .map_err(|e| format!("Failed to insert into analysis_root_dirs db: {}", e))?;

            let id = self.connection.last_insert_rowid();
            Ok(id)
        }
    }

    /// Inserts metadata for a sample and returns the row id
    pub fn insert_sample_metadata(
        &self,
        file_path: &str,
        analysis_root_dir_id: i64,
    ) -> Result<i64, String> {
        self.connection
            .execute(
                "INSERT INTO samples (file_path, analysis_root_dir_id) VALUES (?1, ?2)",
                [&file_path, &analysis_root_dir_id.to_string().as_str()],
            )
            .map_err(|e| format!("Error inserting into sqlite db: {}", e))?;

        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_audio_files(
        &self,
        start_offset: u32,
        limit: u32,
    ) -> Result<Vec<AudioFile>, String> {
        let mut query = self
            .connection
            .prepare(
                format!(
                    "SELECT id, file_path FROM samples WHERE id > {} ORDER BY id LIMIT {}",
                    start_offset, limit
                )
                .as_str(),
            )
            .map_err(|e| format!("Failed to prepare sqlite query: {}", e))?;
        let mut rows = query.query([]).map_err(|e| e.to_string())?;
        let mut files: Vec<AudioFile> = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: i64 = row.get(0).unwrap();
            let path: String = row.get(1).unwrap();
            files.push(AudioFile { id, path });
        }
        Ok(files)
    }
}
