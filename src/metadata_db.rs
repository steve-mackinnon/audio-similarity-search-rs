use std::collections::HashMap;

use crate::{feature::Feature, file_utils};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

pub struct MetadataDatabase {
    connection: Connection,
}

#[derive(Clone, Serialize, Deserialize)]
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
                    file_path TEXT NOT NULL UNIQUE,
                    feature_vector BLOB NOT NULL,
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
        feature_vec: &[f32],
    ) -> Result<i64, String> {
        let serialized_vec = bincode::serialize(feature_vec).map_err(|e| e.to_string())?;
        if self
            .connection
            .execute(
                     "INSERT INTO samples (file_path, analysis_root_dir_id, feature_vector) VALUES (?1, ?2, ?3)",
                   params![
                    &file_path,
                    &analysis_root_dir_id.to_string().as_str(),
                    &serialized_vec,
                ],
            )
            .is_ok()
        {
            Ok(self.connection.last_insert_rowid())
        } else {
            // The execution failure is most likely due to inserting a file_path that already
            // exists. Attempt to get the id for file_path
            let mut query = self
                .connection
                .prepare("SELECT id FROM samples WHERE file_path = ?1")
                .map_err(|e| format!("Failed to prepare sqlite query: {}", e))?;

            let mut rows = query
                .query(rusqlite::params![file_path])
                .map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let id: i64 = row.get(0).unwrap();
                Ok(id)
            } else {
                Err(format!(
                    "Failed to insert or lookup id for sample {}",
                    file_path
                ))
            }
        }
    }

    pub fn list_audio_files(
        &self,
        start_offset: u32,
        limit: u32,
    ) -> Result<Vec<AudioFile>, String> {
        let mut query = self
            .connection
            .prepare("SELECT id, file_path FROM samples WHERE id > ?1 ORDER BY file_path LIMIT ?2")
            .map_err(|e| format!("Failed to prepare sqlite query: {}", e))?;

        let mut rows = query
            .query(rusqlite::params![start_offset, limit])
            .map_err(|e| e.to_string())?;
        let mut files: Vec<AudioFile> = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: i64 = row.get(0).unwrap();
            let path: String = row.get(1).unwrap();
            files.push(AudioFile { id, path });
        }
        Ok(files)
    }

    pub fn get_all_features(&self) -> Result<HashMap<String, Feature>, String> {
        let query = self
            .connection
            .prepare("SELECT file_path, feature_vector, id FROM samples");
        // Return an empty hashmap if the query fails, since this will happen when
        // get_all_features() is called before a metadata db is populated.
        if query.is_err() {
            return Ok(HashMap::new());
        }
        let mut query = query.unwrap();

        let feature_map: HashMap<String, Feature> = query
            .query_map([], |row| {
                let path: String = row.get(0).unwrap();
                let feature_vec: Vec<u8> = row.get(1).unwrap();
                let feature_vec: Vec<f32> = bincode::deserialize(&feature_vec).unwrap();
                let id: i64 = row.get(2).unwrap();
                Ok((path.clone(), Feature::new(feature_vec, path, Some(id))))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|val| {
                if val.is_ok() {
                    return Some(val.unwrap());
                }
                None
            })
            .collect();
        Ok(feature_map)
    }

    pub fn get_audio_files_for_ids(&self, ids: &[u32]) -> Result<Vec<AudioFile>, String> {
        let id_list: String = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT id, file_path FROM samples WHERE id IN ({})",
            id_list
        );

        let mut stmt = self.connection.prepare(&query).map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: i64 = row.get(0).unwrap();
            let path: String = row.get(1).unwrap();
            files.push(AudioFile { id, path });
        }
        // The result of the sql query isn't guaranteed to match the order of ids, which are
        // ranked by most to least similar. So, manually get the AudioFiles into order before
        // returning.
        let ordered_files: Vec<AudioFile> = ids
            .iter()
            .filter_map(|id| Some(files.iter().find(|f| f.id() == *id as i64)?.clone()))
            .collect();
        Ok(ordered_files)
    }
}
