use crate::pasta::PastaFile;
use crate::util::animalnumbers::to_animal_names;
use crate::util::db::insert;
use crate::util::hashids::to_hashids;
use crate::util::misc::{encrypt, encrypt_bytes, is_valid_url};
use crate::util::storage;
use crate::{AppState, Pasta, ARGS};
use actix_multipart::Multipart;
use actix_web::cookie::time::Duration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::error::ErrorBadRequest;
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse, Responder};
use askama::Template;
use bytesize::ByteSize;
use futures::TryStreamExt;
use log::warn;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    args: &'a ARGS,
    has_uploader_cookie: bool,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    args: &'a ARGS,
    status: String,
}

/// Check if request has valid uploader cookie
fn check_uploader_cookie(req: &HttpRequest) -> bool {
    if !ARGS.readonly || ARGS.uploader_password.is_none() {
        return false;
    }
    let expected_token =
        generate_uploader_token(ARGS.uploader_password.as_ref().unwrap().trim());
    req.cookie("uploader_token")
        .map(|c| c.value() == expected_token)
        .unwrap_or(false)
}

#[get("/")]
pub async fn index(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(
        IndexTemplate {
            args: &ARGS,
            has_uploader_cookie: check_uploader_cookie(&req),
        }
        .render()
        .unwrap(),
    )
}

#[get("/{status}")]
pub async fn index_with_status(req: HttpRequest, _param: web::Path<String>) -> HttpResponse {
    // status parameter exists for URL compatibility but is not used in template
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(
        IndexTemplate {
            args: &ARGS,
            has_uploader_cookie: check_uploader_cookie(&req),
        }
        .render()
        .unwrap(),
    )
}

pub fn expiration_to_timestamp(expiration: &str, timenow: i64) -> i64 {
    match expiration {
        "1min" => timenow + 60,
        "10min" => timenow + 60 * 10,
        "1hour" => timenow + 60 * 60,
        "24hour" => timenow + 60 * 60 * 24,
        "3days" => timenow + 60 * 60 * 24 * 3,
        "1week" => timenow + 60 * 60 * 24 * 7,
        "never" => {
            if ARGS.eternal_pasta {
                0
            } else {
                timenow + 60 * 60 * 24 * 7
            }
        }
        _ => {
            log::error!("{}", "Unexpected expiration time!");
            timenow + 60 * 60 * 24 * 7
        }
    }
}

/// Helper function to generate uploader token from password
fn generate_uploader_token(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b"microbin_uploader_salt_2024");
    format!("{:x}", hasher.finalize())
}

#[derive(Deserialize)]
pub struct UploaderLoginForm {
    password: String,
}

/// Show login page
#[get("/login")]
pub async fn login_page() -> HttpResponse {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(
        LoginTemplate {
            args: &ARGS,
            status: String::from(""),
        }
        .render()
        .unwrap(),
    )
}

/// Show login page with status
#[get("/login/{status}")]
pub async fn login_page_with_status(param: web::Path<String>) -> HttpResponse {
    let status = param.into_inner();
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(
        LoginTemplate {
            args: &ARGS,
            status,
        }
        .render()
        .unwrap(),
    )
}

/// Handle login form submission
#[post("/login")]
pub async fn login_submit(form: web::Form<UploaderLoginForm>) -> HttpResponse {
    if !ARGS.readonly || ARGS.uploader_password.is_none() {
        return HttpResponse::Found()
            .append_header(("Location", format!("{}/", ARGS.public_path_as_str())))
            .finish();
    }

    let expected_password = ARGS.uploader_password.as_ref().unwrap().trim();

    if form.password.trim() == expected_password {
        // Password correct, set cookie and redirect to home
        let token = generate_uploader_token(expected_password);

        // Determine if we should use secure cookies based on public_path
        let use_secure = ARGS.public_path_as_str().starts_with("https://");
        log::info!(
            "Uploader login successful, setting cookie (secure={}, public_path={})",
            use_secure,
            ARGS.public_path_as_str()
        );

        let cookie = Cookie::build("uploader_token", token)
            .path("/")
            .max_age(Duration::days(365 * 3))
            .secure(use_secure)
            .same_site(if use_secure { SameSite::Strict } else { SameSite::Lax })
            .http_only(true)
            .finish();
        HttpResponse::Found()
            .cookie(cookie)
            .append_header(("Location", format!("{}/", ARGS.public_path_as_str())))
            .finish()
    } else {
        // Password incorrect, show login page with error
        log::warn!("Uploader login failed: incorrect password");
        HttpResponse::Found()
            .append_header(("Location", format!("{}/login/incorrect", ARGS.public_path_as_str())))
            .finish()
    }
}

/// receives a file through http Post on url /upload/a-b-c with a, b and c
/// different animals. The client sends the post in response to a form.
// TODO: form field order might need to be changed. In my testing the attachment
// data is nestled between password encryption key etc <21-10-24, dvdsk>
pub async fn create(
    req: HttpRequest,
    data: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let mut pastas = data.pastas.lock().unwrap();

    let timenow: i64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => {
            log::error!("SystemTime before UNIX EPOCH!");
            0
        }
    } as i64;

    let mut new_pasta = Pasta {
        id: rand::thread_rng().gen::<u16>() as u64,
        content: String::from(""),
        file: None,
        extension: String::from(""),
        private: false,
        readonly: false,
        editable: ARGS.editable,
        encrypt_server: false,
        encrypted_key: Some(String::from("")),
        encrypt_client: false,
        created: timenow,
        read_count: 0,
        burn_after_reads: 0,
        last_read: timenow,
        pasta_type: String::from(""),
        expiration: expiration_to_timestamp(&ARGS.default_expiry, timenow),
    };

    let mut random_key: String = String::from("");
    let mut plain_key: String = String::from("");
    let mut uploader_password = String::from("");
    let mut pending_file_data: Option<(PastaFile, Vec<u8>)> = None;

    while let Some(mut field) = payload.try_next().await? {
        let Some(field_name) = field.name() else {
            continue;
        };
        match field_name {
            "uploader_password" => {
                while let Some(chunk) = field.try_next().await? {
                    uploader_password
                        .push_str(std::str::from_utf8(&chunk).unwrap().to_string().as_str());
                }
                continue;
            }
            "random_key" => {
                while let Some(chunk) = field.try_next().await? {
                    random_key = std::str::from_utf8(&chunk).unwrap().to_string();
                }
                continue;
            }
            "privacy" => {
                while let Some(chunk) = field.try_next().await? {
                    let privacy = std::str::from_utf8(&chunk).unwrap();
                    new_pasta.private = match privacy {
                        "public" => false,
                        _ => true,
                    };
                    new_pasta.readonly = match privacy {
                        "readonly" => true,
                        _ => false,
                    };
                    new_pasta.encrypt_client = match privacy {
                        "secret" => true,
                        _ => false,
                    };
                    new_pasta.encrypt_server = match privacy {
                        "private" => true,
                        "secret" => true,
                        _ => false,
                    };
                }
            }
            "plain_key" => {
                while let Some(chunk) = field.try_next().await? {
                    plain_key = std::str::from_utf8(&chunk).unwrap().to_string();
                }
                continue;
            }
            "encrypted_random_key" => {
                while let Some(chunk) = field.try_next().await? {
                    new_pasta.encrypted_key =
                        Some(std::str::from_utf8(&chunk).unwrap().to_string());
                }
                continue;
            }
            "expiration" => {
                while let Some(chunk) = field.try_next().await? {
                    new_pasta.expiration =
                        expiration_to_timestamp(std::str::from_utf8(&chunk).unwrap(), timenow);
                }

                continue;
            }
            "burn_after" => {
                while let Some(chunk) = field.try_next().await? {
                    new_pasta.burn_after_reads = match std::str::from_utf8(&chunk).unwrap() {
                        "1" => 1,
                        "10" => 10,
                        "100" => 100,
                        "1000" => 1000,
                        "10000" => 10000,
                        "0" => 0,
                        _ => {
                            log::error!("{}", "Unexpected burn after value!");
                            0
                        }
                    };
                }

                continue;
            }
            "content" => {
                let mut content = String::from("");
                while let Some(chunk) = field.try_next().await? {
                    content.push_str(std::str::from_utf8(&chunk).unwrap().to_string().as_str());
                }
                if !content.is_empty() {
                    new_pasta.content = content;

                    new_pasta.pasta_type = if is_valid_url(new_pasta.content.as_str()) {
                        String::from("url")
                    } else {
                        String::from("text")
                    };
                }
                continue;
            }
            "syntax_highlight" => {
                while let Some(chunk) = field.try_next().await? {
                    new_pasta.extension = std::str::from_utf8(&chunk).unwrap().to_string();
                }
                continue;
            }
            "file" => {
                if ARGS.no_file_upload {
                    continue;
                }

                let path = field.content_disposition().and_then(|cd| cd.get_filename());

                let path = match path {
                    Some("") => continue,
                    Some(p) => p,
                    None => continue,
                };

                let mut file = match PastaFile::from_unsanitized(path) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Unsafe file name: {e:?}");
                        continue;
                    }
                };

                let mut file_data: Vec<u8> = Vec::new();
                while let Some(chunk) = field.try_next().await? {
                    file_data.extend_from_slice(&chunk);
                    if (new_pasta.encrypt_server
                        && file_data.len() > ARGS.max_file_size_encrypted_mb * 1024 * 1024)
                        || file_data.len() > ARGS.max_file_size_unencrypted_mb * 1024 * 1024
                    {
                        return Err(ErrorBadRequest("File exceeded size limit."));
                    }
                }

                file.size = ByteSize::b(file_data.len() as u64);

                // Store file data temporarily for later processing (after we know encryption settings)
                pending_file_data = Some((file, file_data));
                new_pasta.pasta_type = String::from("text");
            }
            field => {
                log::error!("Unexpected multipart field:  {}", field);
            }
        }
    }

    // Track if we need to set the uploader cookie
    let mut should_set_uploader_cookie = false;

    if ARGS.readonly && ARGS.uploader_password.is_some() {
        let expected_password = ARGS.uploader_password.as_ref().unwrap().trim();
        let expected_token = generate_uploader_token(expected_password);

        // Check if valid cookie exists
        let has_valid_cookie = req
            .cookie("uploader_token")
            .map(|c| c.value() == expected_token)
            .unwrap_or(false);

        if has_valid_cookie {
            // Cookie is valid, allow upload
            log::info!("Uploader authenticated via cookie");
        } else if uploader_password.trim() == expected_password {
            // Password matches, set cookie for future requests
            should_set_uploader_cookie = true;
            log::info!("Uploader authenticated via password, will set cookie");
        } else {
            log::warn!(
                "Uploader password mismatch. Input length: {}, Expected length: {}",
                uploader_password.trim().len(),
                expected_password.len()
            );
            return Ok(HttpResponse::Found()
                .append_header((
                    "Location",
                    format!("{}/incorrect", ARGS.public_path_as_str()),
                ))
                .finish());
        }
    }

    let id = new_pasta.id;

    if plain_key != *"" && new_pasta.readonly {
        new_pasta.encrypted_key = Some(encrypt(id.to_string().as_str(), &plain_key));
    }

    if new_pasta.encrypt_server && !new_pasta.readonly && new_pasta.content != *"" {
        if new_pasta.encrypt_client {
            new_pasta.content = encrypt(&new_pasta.content, &random_key);
        } else {
            new_pasta.content = encrypt(&new_pasta.content, &plain_key);
        }
    }

    // Process pending file data - encrypt in memory if needed, then save
    if let Some((mut file, file_data)) = pending_file_data {
        let pasta_id = new_pasta.id_as_animals();
        let display_name = file.display_name().to_string();

        if new_pasta.encrypt_server && !new_pasta.readonly {
            // Encrypt file data in memory
            let key = if new_pasta.encrypt_client {
                &random_key
            } else {
                &plain_key
            };
            let encrypted_data = encrypt_bytes(&file_data, key);

            // Save encrypted file directly as data.enc
            let storage_path = storage::generate_storage_path(&pasta_id, "data.enc");
            storage::save_file(&pasta_id, &storage_path, &encrypted_data)
                .await
                .expect("Failed to save encrypted file");

            // Set file name with appropriate prefix for encrypted files
            if ARGS.s3_enabled() {
                file.name = format!("s3:{}", display_name);
            } else {
                file.name = display_name;
            }
        } else {
            // Save unencrypted file directly
            let storage_path = storage::generate_storage_path(&pasta_id, &file.name);
            storage::save_file(&pasta_id, &storage_path, &file_data)
                .await
                .expect("Failed to save file");

            // Update file name with S3 path if using S3
            if ARGS.s3_enabled() {
                file.name = storage_path;
            }
        }

        new_pasta.file = Some(file);
    }

    let encrypt_server = new_pasta.encrypt_server;

    pastas.push(new_pasta);

    for (_, pasta) in pastas.iter().enumerate() {
        if pasta.id == id {
            insert(Some(&pastas), Some(pasta));
        }
    }

    let slug = if ARGS.hash_ids {
        to_hashids(id)
    } else {
        to_animal_names(id)
    };

    // Build uploader cookie if needed (valid for 3 years, HTTPS only, SameSite Strict)
    let uploader_cookie = if should_set_uploader_cookie {
        let token = generate_uploader_token(ARGS.uploader_password.as_ref().unwrap().trim());
        Some(
            Cookie::build("uploader_token", token)
                .path("/")
                .max_age(Duration::days(365 * 3))
                .secure(true)
                .same_site(SameSite::Strict)
                .http_only(true)
                .finish(),
        )
    } else {
        None
    };

    if encrypt_server {
        let mut builder = HttpResponse::Found();
        builder.append_header(("Location", format!("/auth/{}/success", slug)));
        if let Some(cookie) = uploader_cookie {
            builder.cookie(cookie);
        }
        Ok(builder.finish())
    } else {
        // Generate time-limited token for initial view using Hashids
        let timenow = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiry = timenow + 15; // 15 seconds validity

        // Use global HARSH instance
        let encoded_token = crate::util::hashids::HARSH.encode(&[expiry, id]);

        let mut builder = HttpResponse::Found();
        builder.append_header((
            "Location",
            format!("{}/upload/{}", ARGS.public_path_as_str(), slug),
        ));
        builder.cookie(
            Cookie::build("owner_token", encoded_token)
                .path("/")
                .max_age(Duration::seconds(15))
                .finish(),
        );
        if let Some(cookie) = uploader_cookie {
            builder.cookie(cookie);
        }
        Ok(builder.finish())
    }
}
