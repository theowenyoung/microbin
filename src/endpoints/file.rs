use std::path::PathBuf;

use crate::args::ARGS;
use crate::util::auth;
use crate::util::hashids::to_u64 as hashid_to_u64;
use crate::util::misc::{decrypt_bytes, remove_expired};
use crate::util::storage;
use crate::util::animalnumbers::to_u64;
use crate::AppState;
use actix_multipart::Multipart;
use actix_web::http::header;
use actix_web::{get, post, web, Error, HttpResponse};

#[post("/secure_file/{id}")]
pub async fn post_secure_file(
    data: web::Data<AppState>,
    id: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse, Error> {
    // get access to the pasta collection
    let mut pastas = data.pastas.lock().unwrap();

    let id = if ARGS.hash_ids {
        hashid_to_u64(&id).unwrap_or(0)
    } else {
        to_u64(&id.into_inner()).unwrap_or(0)
    };

    // remove expired pastas (including this one if needed)
    remove_expired(&mut pastas);

    // find the index of the pasta in the collection based on u64 id
    let mut index: usize = 0;
    let mut found: bool = false;
    for (i, pasta) in pastas.iter().enumerate() {
        if pasta.id == id {
            index = i;
            found = true;
            break;
        }
    }

    let password = auth::password_from_multipart(payload).await?;
    log::info!("Received password/key length: {}, first chars: {}...",
        password.len(),
        password.chars().take(8).collect::<String>());

    if found {
        if let Some(ref pasta_file) = pastas[index].file {
            let pasta_id = pastas[index].id_as_animals();
            let display_name = pasta_file.display_name().to_string();

            log::info!("Secure file download: pasta_id={}, file_name={}, is_s3_encrypted={}",
                pasta_id, pasta_file.name(), pasta_file.is_s3_encrypted());

            // Determine storage path for encrypted file (data.enc)
            let storage_path = if pasta_file.is_s3_encrypted() {
                // Encrypted file stored in S3
                format!("s3://attachments/{}/data.enc", pasta_id)
            } else {
                // Encrypted file stored locally
                "data.enc".to_string()
            };

            log::info!("Fetching encrypted file from: {}", storage_path);

            // Get encrypted file data from storage
            let encrypted_data = storage::get_file(&pasta_id, &storage_path)
                .await
                .map_err(|e| {
                    log::error!("Failed to get file: {}", e);
                    actix_web::error::ErrorNotFound(e)
                })?;

            log::info!("Got encrypted data, size={} bytes, attempting decrypt", encrypted_data.len());

            // Decrypt the data
            let decrypted_data = decrypt_bytes(&encrypted_data, &password)
                .map_err(|e| {
                    log::error!("Failed to decrypt: {:?}", e);
                    actix_web::error::ErrorUnauthorized("Failed to decrypt file")
                })?;

            // Set the content type based on the file extension
            let content_type = mime_guess::from_path(&display_name)
                .first_or_octet_stream()
                .to_string();

            // Create a response with the decrypted data
            let response = HttpResponse::Ok()
                .content_type(content_type)
                .append_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", display_name),
                ))
                .body(decrypted_data);
            return Ok(response);
        }
    }
    Ok(HttpResponse::NotFound().finish())
}

#[get("/file/{id}")]
pub async fn get_file(
    request: actix_web::HttpRequest,
    id: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // get access to the pasta collection
    let mut pastas = data.pastas.lock().unwrap();

    let id_intern = if ARGS.hash_ids {
        hashid_to_u64(&id).unwrap_or(0)
    } else {
        to_u64(&id.into_inner()).unwrap_or(0)
    };

    // remove expired pastas (including this one if needed)
    remove_expired(&mut pastas);

    // find the index of the pasta in the collection based on u64 id
    let mut index: usize = 0;
    let mut found: bool = false;
    for (i, pasta) in pastas.iter().enumerate() {
        if pasta.id == id_intern {
            index = i;
            found = true;
            break;
        }
    }

    if found {
        if let Some(ref pasta_file) = pastas[index].file {
            if pastas[index].encrypt_server {
                return Ok(HttpResponse::Found()
                    .append_header((
                        "Location",
                        format!("/auth_file/{}", pastas[index].id_as_animals()),
                    ))
                    .finish());
            }

            let pasta_id = pastas[index].id_as_animals();
            let storage_path = pasta_file.name().to_string();
            let display_name = pasta_file.display_name().to_string();

            if pasta_file.is_s3() {
                // File is stored in S3
                let file_data = storage::get_file(&pasta_id, &storage_path)
                    .await
                    .map_err(|e| actix_web::error::ErrorNotFound(e))?;

                let content_type = mime_guess::from_path(&display_name)
                    .first_or_octet_stream()
                    .to_string();

                return Ok(HttpResponse::Ok()
                    .content_type(content_type)
                    .append_header((
                        "Content-Disposition",
                        format!("attachment; filename=\"{}\"", display_name),
                    ))
                    .body(file_data));
            } else {
                // File is stored locally - use NamedFile for streaming
                let file_path = format!(
                    "{}/attachments/{}/{}",
                    ARGS.data_dir,
                    pasta_id,
                    storage_path
                );
                let file_path = PathBuf::from(file_path);

                let file_response = actix_files::NamedFile::open(file_path)?;
                let file_response = file_response.set_content_disposition(header::ContentDisposition {
                    disposition: header::DispositionType::Attachment,
                    parameters: vec![header::DispositionParam::Filename(display_name)],
                });
                return Ok(file_response.into_response(&request));
            }
        }
    }

    Ok(HttpResponse::NotFound().finish())
}
