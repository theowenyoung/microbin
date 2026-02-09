use clap::Parser;
use lazy_static::lazy_static;
use serde::Serialize;
use std::convert::Infallible;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

lazy_static! {
    pub static ref ARGS: Args = Args::parse();
}

#[derive(Parser, Debug, Clone, Serialize)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(long, env = "MICROBIN_BASIC_AUTH_USERNAME")]
    pub auth_basic_username: Option<String>,

    #[clap(long, env = "MICROBIN_BASIC_AUTH_PASSWORD")]
    pub auth_basic_password: Option<String>,

    #[clap(long, env = "MICROBIN_ADMIN_USERNAME", default_value = "admin")]
    pub auth_admin_username: String,

    #[clap(long, env = "MICROBIN_ADMIN_PASSWORD", default_value = "m1cr0b1n")]
    pub auth_admin_password: String,

    #[clap(long, env = "MICROBIN_EDITABLE")]
    pub editable: bool,

    #[clap(long, env = "MICROBIN_FOOTER_TEXT")]
    pub footer_text: Option<String>,

    #[clap(long, env = "MICROBIN_HIDE_FOOTER")]
    pub hide_footer: bool,

    #[clap(long, env = "MICROBIN_HIDE_HEADER")]
    pub hide_header: bool,

    #[clap(long, env = "MICROBIN_HIDE_LOGO", default_value_t = true)]
    pub hide_logo: bool,

    #[clap(long, env = "MICROBIN_NO_LISTING")]
    pub no_listing: bool,

    #[clap(long, env = "MICROBIN_HIGHLIGHTSYNTAX", default_value_t = true)]
    pub highlightsyntax: bool,

    #[clap(long, env = "MICROBIN_DEFAULT_SYNTAX", default_value = "md")]
    pub default_syntax: String,

    #[clap(long, env = "MICROBIN_RENDER_MARKDOWN", default_value_t = true)]
    pub render_markdown: bool,

    #[clap(long, env = "MICROBIN_RENDER_HTML", default_value_t = true)]
    pub render_html: bool,

    #[clap(short, long, env = "MICROBIN_PORT", default_value_t = 9018)]
    pub port: u16,

    #[clap(short, long, env="MICROBIN_BIND", default_value_t = IpAddr::from([0, 0, 0, 0]))]
    pub bind: IpAddr,

    #[clap(long, env = "MICROBIN_PRIVATE", default_value_t = true)]
    pub private: bool,

    #[clap(long, env = "MICROBIN_DEFAULT_PRIVACY", default_value = "public")]
    pub default_privacy: String,

    #[clap(long, env = "MICROBIN_PURE_HTML")]
    pub pure_html: bool,

    #[clap(long, env = "MICROBIN_JSON_DB")]
    pub json_db: bool,

    #[clap(long, env = "MICROBIN_PUBLIC_PATH")]
    pub public_path: Option<PublicUrl>,

    #[clap(long, env = "MICROBIN_SHORT_PATH")]
    pub short_path: Option<PublicUrl>,

    #[clap(long, env = "MICROBIN_UPLOADER_PASSWORD")]
    pub uploader_password: Option<String>,

    #[clap(long, env = "MICROBIN_READONLY")]
    pub readonly: bool,

    #[clap(long, env = "MICROBIN_SHOW_READ_STATS", default_value_t = true)]
    pub show_read_stats: bool,

    #[clap(long, env = "MICROBIN_TITLE")]
    pub title: Option<String>,

    #[clap(short, long, env = "MICROBIN_THREADS", default_value_t = 1)]
    pub threads: u8,

    #[clap(short, long, env = "MICROBIN_GC_DAYS", default_value_t = 30)]
    pub gc_days: u16,

    #[clap(long, env = "MICROBIN_ENABLE_BURN_AFTER")]
    pub enable_burn_after: bool,

    #[clap(short, long, env = "MICROBIN_DEFAULT_BURN_AFTER", default_value_t = 0)]
    pub default_burn_after: u16,

    #[clap(long, env = "MICROBIN_WIDE")]
    pub wide: bool,

    #[clap(long, env = "MICROBIN_QR", default_value_t = true)]
    pub qr: bool,

    #[clap(long, env = "MICROBIN_ETERNAL_PASTA", default_value_t = true)]
    pub eternal_pasta: bool,

    #[clap(long, env = "MICROBIN_ENABLE_READONLY")]
    pub enable_readonly: bool,

    #[clap(long, env = "MICROBIN_DEFAULT_EXPIRY", default_value = "1week")]
    pub default_expiry: String,

    #[clap(long, env = "MICROBIN_DATA_DIR", default_value = "microbin_data")]
    pub data_dir: String,

    #[clap(short, long, env = "MICROBIN_NO_FILE_UPLOAD")]
    pub no_file_upload: bool,

    #[clap(long, env = "MICROBIN_CUSTOM_CSS")]
    pub custom_css: Option<String>,

    #[clap(long, env = "MICROBIN_HASH_IDS")]
    pub hash_ids: bool,

    #[clap(long, env = "MICROBIN_LIST_SERVER")]
    pub list_server: bool,

    #[clap(long, env = "MICROBIN_DISABLE_TELEMETRY", default_value_t = true)]
    pub disable_telemetry: bool,

    #[clap(long, env = "MICROBIN_DISABLE_UPDATE_CHECKING")]
    pub disable_update_checking: bool,

    #[clap(long, env = "MICROBIN_ENCRYPTION_CLIENT_SIDE", default_value_t = true)]
    pub encryption_client_side: bool,

    #[clap(long, env = "MICROBIN_ENCRYPTION_SERVER_SIDE", default_value_t = true)]
    pub encryption_server_side: bool,

    #[clap(
        long,
        env = "MICROBIN_MAX_FILE_SIZE_ENCRYPTED_MB",
        default_value_t = 1024
    )]
    pub max_file_size_encrypted_mb: usize,

    #[clap(
        long,
        env = "MICROBIN_MAX_FILE_SIZE_UNENCRYPTED_MB",
        default_value_t = 2048
    )]
    pub max_file_size_unencrypted_mb: usize,

    // S3 storage options
    #[clap(long, env = "MICROBIN_S3_ENDPOINT")]
    pub s3_endpoint: Option<String>,

    #[clap(long, env = "MICROBIN_S3_BUCKET")]
    pub s3_bucket: Option<String>,

    #[clap(long, env = "MICROBIN_S3_ACCESS_KEY")]
    pub s3_access_key: Option<String>,

    #[clap(long, env = "MICROBIN_S3_SECRET_KEY")]
    pub s3_secret_key: Option<String>,

    #[clap(long, env = "MICROBIN_S3_REGION", default_value = "us-west-000")]
    pub s3_region: String,
}

impl Args {
    pub fn public_path_as_str(&self) -> String {
        if self.public_path.is_some() {
            self.public_path.as_ref().unwrap().to_string()
        } else {
            String::from("")
        }
    }

    pub fn short_path_as_str(&self) -> String {
        if self.short_path.is_some() {
            self.short_path.as_ref().unwrap().to_string()
        } else if self.public_path.is_some() {
            self.public_path.as_ref().unwrap().to_string()
        } else {
            String::from("")
        }
    }

    pub fn without_secrets(self) -> Args {
        Args {
            auth_basic_username: None,
            auth_basic_password: None,
            auth_admin_username: String::from(""),
            auth_admin_password: String::from(""),
            editable: self.editable,
            footer_text: self.footer_text,
            hide_footer: self.hide_footer,
            hide_header: self.hide_header,
            hide_logo: self.hide_logo,
            no_listing: self.no_listing,
            highlightsyntax: self.highlightsyntax,
            default_syntax: self.default_syntax,
            render_markdown: self.render_markdown,
            render_html: self.render_html,
            port: self.port,
            bind: self.bind,
            private: self.private,
            default_privacy: self.default_privacy,
            pure_html: self.pure_html,
            json_db: self.json_db,
            public_path: self.public_path,
            short_path: self.short_path,
            uploader_password: None,
            readonly: self.readonly,
            show_read_stats: self.show_read_stats,
            title: self.title,
            list_server: self.list_server,
            threads: self.threads,
            gc_days: self.gc_days,
            enable_burn_after: self.enable_burn_after,
            default_burn_after: self.default_burn_after,
            wide: self.wide,
            qr: self.qr,
            eternal_pasta: self.eternal_pasta,
            enable_readonly: self.enable_readonly,
            default_expiry: self.default_expiry,
            data_dir: String::from(""),
            no_file_upload: self.no_file_upload,
            custom_css: self.custom_css,
            hash_ids: self.hash_ids,
            disable_telemetry: self.disable_telemetry,
            encryption_client_side: self.encryption_client_side,
            encryption_server_side: self.encryption_server_side,
            max_file_size_encrypted_mb: self.max_file_size_encrypted_mb,
            max_file_size_unencrypted_mb: self.max_file_size_unencrypted_mb,
            disable_update_checking: self.disable_update_checking,
            s3_endpoint: None,
            s3_bucket: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_region: String::from(""),
        }
    }

    pub fn s3_enabled(&self) -> bool {
        self.s3_endpoint.is_some()
            && self.s3_bucket.is_some()
            && self.s3_access_key.is_some()
            && self.s3_secret_key.is_some()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUrl(pub String);

impl fmt::Display for PublicUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PublicUrl {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = s.strip_suffix('/').unwrap_or(s).to_owned();
        Ok(PublicUrl(uri))
    }
}
