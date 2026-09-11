use s3_presign::{Bucket, Credentials, Presigner};
use worker::{Env, Error, Result};

#[derive(Clone)]
pub struct R2Creds {
    access_key_id: String,
    secret_access_key: String,
    /// `<account_id>.r2.cloudflarestorage.com`
    endpoint: String,
    bucket_name: String,
}

impl R2Creds {
    /// Load the credentials from Worker secrets — set these with:
    ///   wrangler secret put R2_ACCESS_KEY_ID
    ///   wrangler secret put R2_SECRET_ACCESS_KEY
    ///   wrangler secret put R2_ACCOUNT_ID
    ///   wrangler secret put R2_BUCKET_NAME
    pub fn from_env(env: &Env) -> Result<Self> {
        Ok(Self {
            access_key_id: env.secret("R2_ACCESS_KEY_ID")?.to_string(),
            secret_access_key: env.secret("R2_SECRET_ACCESS_KEY")?.to_string(),
            endpoint: format!("{}.r2.cloudflarestorage.com", env.secret("R2_ACCOUNT_ID")?),
            bucket_name: env.secret("R2_BUCKET_NAME")?.to_string(),
        })
    }

    pub fn presign_get(&self, key: &str, expires_secs: i64) -> Result<String> {
        let credentials = Credentials::new(&self.access_key_id, &self.secret_access_key, None);
        // R2's S3-compatible API requires region "auto" in the SigV4 scope; see
        // https://developers.cloudflare.com/r2/api/s3/presigned-urls/
        let s3_bucket = Bucket::new_with_root("auto", &self.bucket_name, &self.endpoint);
        let mut presigner = Presigner::from_bucket(credentials, &s3_bucket);
        let presigner = presigner.use_path_style();
        presigner
            .get(key, expires_secs)
            .ok_or_else(|| Error::RustError("Presign failed".into()))
    }
}
