# douteux

A URL shadifier — a Cloudflare Worker that shortens URLs and hosts file downloads behind short slugs.

Built with Rust, [axum](https://github.com/tokio-rs/axum), and the [workers-rs](https://github.com/cloudflare/workers-rs) crate.

## Features

- **Link redirect** — shorten any URL to a slug; visiting the slug issues a `301` redirect
- **File download** — upload a file; visiting the slug issues a `302` redirect to a presigned R2 URL (valid 1 hour)
- **Auth** — write and delete routes are protected by a Bearer token
- **Storage** — slugs and entry metadata stored in KV; file bytes stored in R2, content-addressed by SHA-256 (identical uploads are deduplicated)

## API

### `GET /:slug`

Resolves a slug. Public, no auth required.

- Redirect entry → `301 Moved Permanently` to the target URL
- File entry → `302 Found` to a presigned R2 download URL (1 hour TTL)
- Unknown slug → `404 Not Found`

---

### `POST /`

Creates a new short link. Requires auth.

**Headers**
```
Authorization: Bearer <token>
```

#### Redirect

```
Content-Type: application/json
```
```json
{ "url": "https://example.com", "pattern": "shady:movie" }
```

`pattern` is optional and takes a `mode` with an optional `:arg` suffix:

| `pattern`     | result                                                        |
|---------------|--------------------------------------------------------------|
| `shorten`     | random slug, 6 chars                                          |
| `shorten:12`  | random slug, 12 chars (clamped to 3–64)                       |
| `shady:movie` | fake torrent release name (`The.Matrix.1999.VOSTFR.1080p...x264-NOVA[A3F9C1D2].mp4`) |
| `shady:crack` | fake software-crack release name (`Adobe.Photoshop.v25.3.1.Multilingual.Incl.Keygen-KEYFORGE[A3F9C1D2].rar`) |
| `shady:pup`   | fake "PC optimizer" / freeware-installer name (`SmartDriverBooster_Pro_Setup_v8.4_x64[A3F9C1D2].exe`) |
| `shady:tape`  | weird / disturbing "recovered video" name (`DO_NOT_WATCH_the_smiling_woman_attic_3am_no_audio_tape_07[A3F9C1D2].avi`) |
| `shady`       | alias for `shady:movie` (the default kind)                    |
| omitted       | default (`shady:movie`)                                       |

An unknown mode or a non-numeric length is a `400`.

**Response `200`**
```json
{ "slug": "aB3xYz" }
```

#### File upload

Any `Content-Type` other than `application/json`. The request body is the raw
file bytes — the worker streams them straight into R2 without buffering, so
`Content-Length` is **required**.

```
Content-Type: <file mime type>
Content-Length: <byte count>
```

Query parameters:
- `sha256` — **required**, lowercase hex SHA-256 of the body. It becomes the R2
  object key (content-addressed storage), and the worker verifies it as the
  bytes stream past — a mismatch is a `400`. Re-uploading identical content is
  deduplicated: the object is written once and later uploads skip straight to
  creating the new slug.
- `filename` — original file name, used for the download's `Content-Disposition`.
  For deduplicated content the first upload's filename wins.
- `pattern` — optional, same values as above.

**Response `200`**
```json
{ "slug": "aB3xYz" }
```

---

### `DELETE /:slug`

Deletes a slug. If the entry is a file, the R2 object is deleted too — unless
another slug still points at the same content-addressed object. Requires auth.

**Headers**
```
Authorization: Bearer <token>
```

**Response** `204 No Content`

---

## Setup

### 1. Install dependencies

```sh
cargo install worker-build
npm install -g wrangler
```

### 2. Create KV namespace and R2 bucket

```sh
wrangler kv namespace create KV
wrangler r2 bucket create <your-bucket-name>
```

Fill in the `id` returned for the KV namespace in `wrangler.toml`:

```toml
[[kv_namespaces]]
binding = "KV"
id = "<your-kv-namespace-id>"

[[r2_buckets]]
binding = "R2"
bucket_name = "<your-bucket-name>"
```

### 3. Create an R2 API token

In the Cloudflare dashboard: **R2 → Manage R2 API tokens → Create API token** with *Object Read & Write* permissions.

### 4. Set Worker secrets

```sh
wrangler secret put AUTH_TOKEN          # shared bearer token for write/delete
wrangler secret put R2_ACCESS_KEY_ID    # R2 API token access key ID
wrangler secret put R2_SECRET_ACCESS_KEY # R2 API token secret access key
wrangler secret put R2_ACCOUNT_ID       # your Cloudflare account ID
wrangler secret put R2_BUCKET_NAME      # your R2 bucket name
```

### 5. Deploy

```sh
wrangler deploy
```

## Development

```sh
# type-check
cargo check -p douteux

# unit tests — slug pattern parsing + query-string / JSON body decoding.
# Run on the host target.
cargo test -p douteux

# local dev (requires wrangler)
wrangler dev
```
