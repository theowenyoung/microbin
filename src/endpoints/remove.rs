use actix_multipart::Multipart;
use actix_web::{get, post, web, Error, HttpResponse};

use crate::args::ARGS;
use crate::endpoints::errors::ErrorTemplate;
use crate::pasta::PastaFile;
use crate::util::animalnumbers::to_u64;
use crate::util::auth;
use crate::util::db::delete;
use crate::util::hashids::to_u64 as hashid_to_u64;
use crate::util::misc::{decrypt, remove_expired};
use crate::util::storage;
use crate::AppState;
use askama::Template;

#[get("/remove/{id}")]
pub async fn remove(data: web::Data<AppState>, id: web::Path<String>) -> HttpResponse {
    let mut pastas = data.pastas.lock().unwrap();

    let id = if ARGS.hash_ids {
        hashid_to_u64(&id).unwrap_or(0)
    } else {
        to_u64(&id.into_inner()).unwrap_or(0)
    };

    for (i, pasta) in pastas.iter().enumerate() {
        if pasta.id == id {
            // if it's encrypted or read-only, it needs password to be deleted
            // OR if it is not editable (public immutable), it needs admin password to be deleted
            if pasta.encrypt_server || pasta.readonly || !pasta.editable {
                return HttpResponse::Found()
                    .append_header((
                        "Location",
                        format!("{}/auth_remove_private/{}", ARGS.public_path_as_str(), pasta.id_as_animals()),
                    ))
                    .finish();
            }

            let pasta_id = pasta.id_as_animals();

            // remove the file using storage abstraction
            if let Some(PastaFile { name, .. }) = &pasta.file {
                let filename = name.clone();
                // Need to drop the lock before await
                drop(pastas);

                if let Err(e) = storage::delete_file(&pasta_id, &filename).await {
                    log::error!("Failed to delete file {}: {}", filename, e);
                }

                // Re-acquire lock
                pastas = data.pastas.lock().unwrap();

                // Find the pasta again (index may have changed)
                let mut new_index = None;
                for (j, p) in pastas.iter().enumerate() {
                    if p.id == id {
                        new_index = Some(j);
                        break;
                    }
                }

                if let Some(idx) = new_index {
                    pastas.remove(idx);
                }

                delete(Some(&pastas), Some(id));

                return HttpResponse::Found()
                    .append_header(("Location", format!("{}/list", ARGS.public_path_as_str())))
                    .finish();
            }

            // remove it from in-memory pasta list
            pastas.remove(i);

            delete(Some(&pastas), Some(id));

            return HttpResponse::Found()
                .append_header(("Location", format!("{}/list", ARGS.public_path_as_str())))
                .finish();
        }
    }

    remove_expired(&mut pastas);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(ErrorTemplate { args: &ARGS }.render().unwrap())
}

#[post("/remove/{id}")]
pub async fn post_remove(
    data: web::Data<AppState>,
    id: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse, Error> {
    let id = if ARGS.hash_ids {
        hashid_to_u64(&id).unwrap_or(0)
    } else {
        to_u64(&id.into_inner()).unwrap_or(0)
    };

    let password = auth::password_from_multipart(payload).await?;

    // First, check if we need to delete a file and collect the info we need
    let file_to_delete: Option<(String, String)>;
    let pasta_animals: String;
    let should_delete: bool;
    let is_protected: bool;
    let redirect_to_upload: bool;

    {
        let mut pastas = data.pastas.lock().unwrap();
        remove_expired(&mut pastas);

        let pasta = pastas.iter().find(|p| p.id == id);

        if pasta.is_none() {
            return Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(ErrorTemplate { args: &ARGS }.render().unwrap()));
        }

        let pasta = pasta.unwrap();
        pasta_animals = pasta.id_as_animals();
        is_protected = pasta.readonly || pasta.encrypt_server || !pasta.editable;

        if !is_protected {
            // Not protected, redirect to upload page
            redirect_to_upload = true;
            should_delete = false;
            file_to_delete = None;
        } else if password.is_empty() {
            // Protected but no password provided
            redirect_to_upload = false;
            should_delete = false;
            file_to_delete = None;
        } else {
            // Check password
            let mut is_password_correct = password == ARGS.auth_admin_password;

            if !is_password_correct && pasta.readonly {
                if let Some(ref encrypted_key) = pasta.encrypted_key {
                    if let Ok(decrypted_key) = decrypt(encrypted_key, &password) {
                        if decrypted_key == id.to_string() {
                            is_password_correct = true;
                        }
                    }
                }
            } else if !is_password_correct && pasta.encrypt_server {
                if decrypt(&pasta.content, &password).is_ok() {
                    is_password_correct = true;
                }
            }

            if is_password_correct {
                redirect_to_upload = false;
                should_delete = true;
                file_to_delete = pasta.file.as_ref().map(|f| {
                    let storage_path = if pasta.encrypt_server {
                        // Encrypted file - determine if S3 or local
                        if f.is_s3_encrypted() {
                            format!("s3://attachments/{}/data.enc", pasta_animals)
                        } else {
                            "data.enc".to_string()
                        }
                    } else {
                        // Non-encrypted - use stored path directly
                        f.name.clone()
                    };
                    (pasta_animals.clone(), storage_path)
                });
            } else {
                redirect_to_upload = false;
                should_delete = false;
                file_to_delete = None;
            }
        }
    } // Lock released here

    if redirect_to_upload {
        return Ok(HttpResponse::Found()
            .append_header((
                "Location",
                format!("{}/upload/{}", ARGS.public_path_as_str(), pasta_animals),
            ))
            .finish());
    }

    if !is_protected || !should_delete {
        return Ok(HttpResponse::Found()
            .append_header((
                "Location",
                format!("{}/auth_remove_private/{}/incorrect", ARGS.public_path_as_str(), pasta_animals),
            ))
            .finish());
    }

    // Delete file if exists
    if let Some((pasta_id, filename)) = file_to_delete {
        if let Err(e) = storage::delete_file(&pasta_id, &filename).await {
            log::error!("Failed to delete file {}: {}", filename, e);
        }
    }

    // Re-acquire lock and remove from list
    {
        let mut pastas = data.pastas.lock().unwrap();
        if let Some(idx) = pastas.iter().position(|p| p.id == id) {
            pastas.remove(idx);
        }
        delete(Some(&pastas), Some(id));
    }

    Ok(HttpResponse::Found()
        .append_header((
            "Location",
            format!("{}/list", ARGS.public_path_as_str()),
        ))
        .finish())
}
