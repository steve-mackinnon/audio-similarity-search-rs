use std::{fs, path::PathBuf};

use directories::ProjectDirs;

pub fn data_directory() -> Result<PathBuf, String> {
    let project_dirs = ProjectDirs::from("com", "steve", "sampleorganizer");
    if project_dirs.is_none() {
        return Err("Failed to create directory for vector database".to_string());
    }
    let project_dirs = project_dirs.unwrap();
    let data_local_dir = project_dirs.data_local_dir();

    if fs::try_exists(data_local_dir).is_err() {
        fs::create_dir(data_local_dir).map_err(|e| e.to_string())?;
    }

    Ok(data_local_dir.to_path_buf())
}

pub fn feature_file_path() -> Result<PathBuf, String> {
    Ok(data_directory()?.join("features"))
}

pub fn db_path() -> Result<PathBuf, String> {
    Ok(data_directory()?.join("data.mdb"))
}

pub fn db_lock_path() -> Result<PathBuf, String> {
    Ok(data_directory()?.join("lock.mdb"))
}
