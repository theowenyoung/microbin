use crate::args::ARGS;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, Region};
use std::fs;
use std::io::Write;
use std::path::Path;

fn get_s3_bucket() -> Result<Box<Bucket>, S3Error> {
    let region = Region::Custom {
        region: ARGS.s3_region.clone(),
        endpoint: ARGS.s3_endpoint.as_ref().unwrap().clone(),
    };

    let credentials = Credentials::new(
        Some(ARGS.s3_access_key.as_ref().unwrap()),
        Some(ARGS.s3_secret_key.as_ref().unwrap()),
        None,
        None,
        None,
    )?;

    let bucket = Bucket::new(ARGS.s3_bucket.as_ref().unwrap(), region, credentials)?
        .with_path_style();

    Ok(bucket)
}

/// Generate the storage path for a file.
/// Returns (storage_name, is_s3) where storage_name includes s3:// prefix if using S3.
pub fn generate_storage_path(pasta_id: &str, filename: &str) -> String {
    if ARGS.s3_enabled() {
        format!("s3://attachments/{}/{}", pasta_id, filename)
    } else {
        filename.to_string()
    }
}

/// Save a file. The `storage_path` should be the value returned by `generate_storage_path`
/// or the `name` field from PastaFile.
pub async fn save_file(pasta_id: &str, storage_path: &str, data: &[u8]) -> Result<(), String> {
    if let Some(s3_path) = storage_path.strip_prefix("s3://") {
        // S3 storage
        let bucket = get_s3_bucket().map_err(|e| format!("Failed to get S3 bucket: {}", e))?;

        bucket
            .put_object(s3_path, data)
            .await
            .map_err(|e| format!("Failed to upload to S3: {}", e))?;

        log::info!("Uploaded file to S3: {}", s3_path);
        Ok(())
    } else {
        // Local storage
        let dir_path = format!("{}/attachments/{}", ARGS.data_dir, pasta_id);
        fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        let file_path = format!("{}/{}", dir_path, storage_path);
        let mut file =
            fs::File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;

        file.write_all(data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }
}

/// Get a file by its storage path.
pub async fn get_file(pasta_id: &str, storage_path: &str) -> Result<Vec<u8>, String> {
    if let Some(s3_path) = storage_path.strip_prefix("s3://") {
        // S3 storage
        let bucket = get_s3_bucket().map_err(|e| format!("Failed to get S3 bucket: {}", e))?;

        let response = bucket
            .get_object(s3_path)
            .await
            .map_err(|e| format!("Failed to get file from S3: {}", e))?;

        Ok(response.to_vec())
    } else {
        // Local storage
        let file_path = format!("{}/attachments/{}/{}", ARGS.data_dir, pasta_id, storage_path);
        fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))
    }
}

/// Delete a file by its storage path.
pub async fn delete_file(pasta_id: &str, storage_path: &str) -> Result<(), String> {
    if let Some(s3_path) = storage_path.strip_prefix("s3://") {
        // S3 storage
        let bucket = get_s3_bucket().map_err(|e| format!("Failed to get S3 bucket: {}", e))?;

        bucket
            .delete_object(s3_path)
            .await
            .map_err(|e| format!("Failed to delete from S3: {}", e))?;

        log::info!("Deleted file from S3: {}", s3_path);
        Ok(())
    } else {
        // Local storage
        let file_path = format!("{}/attachments/{}/{}", ARGS.data_dir, pasta_id, storage_path);

        if Path::new(&file_path).exists() {
            fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }

        let dir_path = format!("{}/attachments/{}", ARGS.data_dir, pasta_id);
        if Path::new(&dir_path).exists() {
            let _ = fs::remove_dir(&dir_path);
        }

        Ok(())
    }
}
