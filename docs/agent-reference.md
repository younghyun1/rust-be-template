# Agent Reference

This document is for future agentic work in this repository. It records the
structure, API surface, code conventions, and domain-specific behavior that are
easy to miss when reading only the file tree.

## Project Identity

- Crate: `rust-be-template`
- Rust edition: `2024`
- Toolchain: `nightly` via `toolchain.toml`
- Main binary: `src/main.rs`
- Web stack: Axum 0.8, Tokio, Tower middleware, `axum-server` with rustls TLS
- Persistence: PostgreSQL with Diesel and `diesel-async` over a `bb8` pool
- OpenAPI: `utoipa` plus `utoipa-swagger-ui`
- Logging: `tracing`, pretty console output, daily JSON log files under `logs/`
- Allocator: `mimalloc`
- Generated file: `src/build_info.rs` is written by `build.rs`; do not edit it
  by hand.

The app is a backend for `cyhdev.com` with auth, blog posts/comments/votes,
country/language dropdown data, i18n text bundles, GeoIP lookup, visitor board
tracking, photography uploads, WASM/demo module hosting, and live chat.

## Top-Level Layout

- `src/main.rs`: process entry point, dotenv loading, tracing setup, rustls crypto
  provider setup, and delegation to server initialization.
- `src/init/`: configuration, `ServerState` construction, cache loading, search
  index setup, and server bootstrap.
- `src/routers/`: route assembly and middleware.
- `src/handlers/`: Axum handlers grouped by API domain.
- `src/domain/`: database-backed domain structs and domain services.
- `src/dto/`: request and response payload types.
- `src/errors/`: project error model and `IntoResponse` implementation.
- `src/jobs/`: background job scheduler and recurring maintenance/auth jobs.
- `src/util/`: cross-cutting helpers for crypto, image processing, email, GeoIP,
  request extraction, system stats, strings, time, and WASM bundles.
- `src/schema.rs`: Diesel schema generated from migrations.
- `migrations/`: Diesel migrations and seed data.
- `i18n/ui/`: file-backed UI text source JSON for `en-US` and `ko-KR`.
- `fe/`: embedded frontend/static asset tree used by `rust-embed`.
- `wasm/`: local WASM-related assets/source area.
- `docs/`: project documentation. This file is the current agent-facing map.

Module `mod.rs` files mostly just expose submodules. New domains should follow
the existing pattern: domain model under `src/domain/<domain>`, DTOs under
`src/dto/{requests,responses}/<domain>`, handlers under `src/handlers/<domain>`,
and route registration in `src/routers/main_router.rs`.

## Startup Flow

1. `main` loads `.env` unless `IS_AWS_ECS` is set.
2. It requires `APP_NAME_VERSION`, configures tracing, and writes daily logs to
   `./logs/<APP_NAME_VERSION>/<APP_NAME_VERSION>.*`.
3. It installs the rustls AWS-LC crypto provider.
4. It spawns `server_init_proc(start)`.
5. `server_init_proc` loads `HOST_IP`, `HOST_PORT`, `CERT_CHAIN_DIR`,
   `PRIV_KEY_DIR`, database config, email config, AWS image-upload credentials,
   GeoIP bundles, search index, fastfetch cache, and app state.
6. State caches are synchronized before serving:
   - blog post metadata and Tantivy search index
   - countries, languages, and currencies
   - file-backed UI text into `i18n_strings`
   - DB i18n cache
   - visitor board data
   - WASM module bundle cache
   - live chat ban and message cache
7. `X_API_KEY` is parsed as a UUID and inserted into in-memory API key state.
8. Background jobs are started.
9. An HTTP redirect listener binds to `127.0.0.1:80`; HTTPS binds to
   `HOST_IP:HOST_PORT`.

TLS is not optional in the normal server path. Local development needs cert
paths unless the bootstrap is changed.

## Important Environment Variables

- `IS_AWS_ECS`: when absent, `.env` is loaded.
- `APP_NAME_VERSION`: used for logs and state.
- `HOST_IP`, `HOST_PORT`: HTTPS bind address.
- `CERT_CHAIN_DIR`, `PRIV_KEY_DIR`: rustls PEM inputs.
- `DB_URL`: preferred database URL unless `DB_HOST` is a Unix socket path.
- `DB_HOST`, `DB_PORT`, `DB_USERNAME`, `DB_PASSWORD`, `DB_NAME`: DB fallback.
- `AWS_SES_SMTP_URL`, `AWS_SES_SMTP_USERNAME`, `AWS_SES_SMTP_ACCESS_KEY`:
  email client configuration.
- `AWS_IMAGE_UPLOAD_KEY`, `AWS_IMAGE_UPLOAD_SECRET_KEY`: S3 client credentials
  used for profile pictures, photography, and WASM thumbnails.
- `AWS_PHOTOGRAPHS_BUCKET` or `AWS_IMAGE_UPLOAD_BUCKET`: photograph deletion
  bucket selection.
- `SEARCH_INDEX_PATH`: optional Tantivy index path, default
  `./data/search_index`.
- `CURR_ENV`: maps to `Local`, `Dev`, `Staging`, or `Prod`; unknown values fall
  back to `Local`, and missing falls back to `Prod`.
- `X_API_KEY`: UUID API key inserted into memory. The API-key middleware exists
  but is currently not applied in the router.

## ServerState

`ServerState` is the shared `Arc` state injected into handlers.

Core fields:

- `pool`: async Postgres connection pool.
- `email_client`: Lettre async SMTP transport.
- `responses_handled`: atomic request counter.
- `deployment_environment`: environment enum used by cookies, Swagger gating,
  and visitor logging.
- `request_client`: shared `reqwest::Client` with `cyhdev.com` user agent.

In-memory caches:

- `session_map`: `scc::HashMap<Uuid, Session>`.
- `blog_posts_cache`: post metadata keyed by post UUID.
- `blog_post_slug_cache`: normalized slug to post UUID.
- `blog_post_order_cache`: `RwLock<Vec<Uuid>>` ordered by newest created time.
- `search_index`: disk-backed Tantivy index for blog title and tags.
- `geo_ip_db`: decompressed IPv4 and IPv6 GeoIP bundles.
- `visitor_board_map` and `visitor_log_buffer`: visitor aggregation.
- `api_keys_set`: in-memory API keys.
- `country_map`, `languages_map`, `currency_map`: cached reference data.
- `i18n_cache`: indexed i18n rows.
- `system_info_state`: CPU/memory snapshots.
- `fastfetch`: cached host information.
- `wasm_module_cache`: pre-compressed bundle bytes keyed by module UUID.
- `live_chat_cache`: message timeline, bans, typing state, connected clients,
  rate state, and broadcast channel.

Conventions:

- Use `state.get_conn().await` for database access.
- Drop DB connections before CPU-heavy or cache-heavy work when practical.
- Cache mutation helpers live under `src/init/state/server_state/*.rs`.
- `scc` maps are used for highly concurrent caches. Use their async APIs rather
  than wrapping them in extra locks.
- Caches are not a passive optimization. Several handlers serve from cache and
  then decorate from DB, so writes must keep caches coherent.

## Routing and Middleware

All primary route registration is in `src/routers/main_router.rs`.

Shared API layers:

- `is_logged_in_middleware`: attaches `AuthStatus` and optional `AuthSession` to
  every API request.
- `log_middleware`: increments response count, extracts client IP, assigns or
  propagates `x-request-id`, adds build headers, logs completion, and enqueues
  visitor logs in production.
- `DefaultBodyLimit`: 150 MB.
- `GovernorLayer`: global rate limiter, configured with 1024 burst and
  replenishment every 63 ms.
- `CorsLayer::very_permissive()`.
- Response compression: zstd and gzip.

Access tiers:

- Public router: no required authenticated session.
- Protected router: `auth_middleware`, which requires a valid `session_id`
  cookie and verified email.
- Superuser router: `auth_middleware` plus `require_superuser_middleware`.

`require_superuser_middleware` requires `RoleType::Younghyun`; despite the
generic `RoleRequirement::AtLeast` name, the current superuser route layer is
effectively owner-only.

Swagger UI:

- `/swagger-ui`
- `/api-docs/openapi.json`
- Always mounted. In `Prod`, the Swagger router is protected by auth and
  superuser middleware.
- `src/docs.rs` must be manually updated for OpenAPI. Utoipa only exposes
  handlers listed in `#[openapi(paths(...))]`.
- Current OpenAPI registration trails the router in some areas, especially
  WebSocket endpoints and WASM module routes. Treat `main_router.rs` as the
  source of truth for API surface.

Static assets:

- Final router uses `static_asset_handler` as fallback.
- Frontend assets are embedded from `fe/` through `rust-embed`.

## API Surface

Public HTTP routes:

- `GET /api/healthcheck/server`
- `GET /api/healthcheck/state`
- `GET /api/healthcheck/fastfetch`
- `GET /api/dropdown/language`
- `GET /api/dropdown/language/{language_id}`
- `GET /api/dropdown/country`
- `GET /api/dropdown/country/{country_id}`
- `GET /api/dropdown/country/{country_id}/subdivision`
- `GET /api/visitor-board`
- `GET /api/geolocate/{ip_address}`
- `GET /api/geo-ip-info/me`
- `GET /api/geo-ip-info/{ip_address}`
- `POST /api/auth/signup`
- `GET /api/auth/me`
- `GET /api/auth/is-superuser`
- `POST /api/auth/check-if-user-exists`
- `POST /api/auth/login`
- `POST /api/auth/reset-password-request`
- `POST /api/auth/reset-password`
- `GET /api/auth/verify-user-email`
- `GET /api/users/{user_name}`
- `GET /api/blog/posts`
- `GET /api/blog/posts/{post_id}`
- `GET /api/blog/search`
- `GET /api/live-chat/messages`
- `GET /api/live-chat/cache-stats`
- `GET /api/i18n/ui-text`
- `GET /api/photographs/get`
- `GET /api/wasm-modules`
- `GET /api/wasm-modules/{wasm_module_id}/wasm`

Public WebSocket routes:

- `GET /ws/host-stats`
- `GET /ws/live-chat`

Authenticated routes:

- `POST /api/auth/logout`
- `POST /api/user/upload-profile-picture`
- `POST /api/blog/{post_id}/vote`
- `DELETE /api/blog/{post_id}/vote`
- `POST /api/blog/{post_id}/{comment_id}/vote`
- `DELETE /api/blog/{post_id}/{comment_id}/vote`
- `DELETE /api/blog/{post_id}/{comment_id}`
- `PATCH /api/blog/{post_id}/{comment_id}`
- `DELETE /api/blog/{post_id}`
- `POST /api/blog/{post_id}/comment`

Superuser routes:

- `GET /api/admin/sync-i18n-cache`
- `POST /api/blog/posts`
- `PATCH /api/blog/{post_id}`
- `POST /api/photographs/upload`
- `DELETE /api/photographs/delete`
- `POST /api/wasm-modules`
- `PATCH /api/wasm-modules/{wasm_module_id}`
- `POST /api/wasm-modules/{wasm_module_id}/assets`
- `DELETE /api/wasm-modules/{wasm_module_id}`

The route names are mostly REST-like but not uniformly so. For example,
photographs use `/api/photographs/get` and `/api/photographs/delete`, while
blog uses resource-like paths.

## Handler Conventions

Most JSON handlers follow this shape:

```rust
pub async fn handler(
    Extension(...): Extension<...>,
    State(state): State<Arc<ServerState>>,
    Query(request): Query<RequestDto>,
    Json(request): Json<RequestDto>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    ...
    Ok(http_resp(ResponseDto { ... }, (), start))
}
```

Key conventions:

- Use `HandlerResponse<T> = Result<T, CodeErrorResp>`.
- Start timing with `util::time::now::tokio_now()`.
- Return success with `http_resp(data, meta, start)`.
- Return success plus cookies with `http_resp_with_cookies(...)`.
- Convert DB/pool/domain errors with `code_err(CodeError::..., e)`.
- Prefer explicit request/response DTOs under `src/dto`.
- Add `#[utoipa::path(...)]` to HTTP handlers intended for Swagger.
- Then add the handler and schema types to `src/docs.rs`.

Multipart handlers use `axum::extract::Multipart` and validate field names
manually. Large CPU/image/bundle work is moved into `tokio::task::spawn_blocking`.

## Response and Error Model

Successful JSON responses serialize as:

```json
{
  "success": true,
  "data": {},
  "meta": {
    "time_to_process": "...",
    "timestamp": "...",
    "metadata": {}
  }
}
```

`metadata` is often `()`, which serializes as `null`.

Errors use `CodeError` constants and serialize only:

```json
{
  "success": false,
  "error_code": 0,
  "message": "..."
}
```

`http_status_code`, `error_message`, and `log_level` are skipped in the JSON
body. They are still used internally. `CodeErrorResp::into_response` attaches a
`CodeErrorLogContext` response extension so `log_middleware` can log the chosen
status, application error code, public message, private detail, and log level.

When adding errors:

- Add a new `CodeError` constant in `src/errors/code_error.rs`.
- Keep `error_code` stable once exposed.
- Choose `log_level` deliberately. Expected client mistakes are often `INFO`;
  authorization misuse is usually `WARN`; server or data failures are `ERROR`.

## Authentication, Sessions, and Roles

Auth is cookie-based:

- Login validates email and password form, verifies Argon2 password hash, removes
  any old `session_id` cookie session if present, creates a new in-memory
  session, and sets a secure, http-only `session_id` cookie.
- Session duration defaults to one hour.
- Sessions live only in memory in `ServerState.session_map`.
- `auth_middleware` requires:
  - parsable `session_id`
  - session present in memory
  - `created_at < now < expires_at`
  - verified email
- `is_logged_in_middleware` is softer and only populates login context when a
  valid session exists. Public handlers use it to decorate results or include
  unpublished content for superusers.

Role model:

- `RoleType::Younghyun = 0`
- `RoleType::Moderator = 1`
- `RoleType::User = 2`
- `RoleType::Guest = 3`

The discriminant values are not the permission order. `access_level()` gives
Younghyun 3, Moderator 2, User 1, Guest 0. Fixed UUID constants map DB role IDs
to enum values. New users are assigned `RoleType::User`.

`RoleType::is_superuser()` is true only for `Younghyun`.

Password and user validation:

- Passwords must be at least 8 characters and contain lowercase, uppercase, and
  ASCII digit characters.
- Usernames must be non-empty, length at most 20 bytes, and all characters must
  satisfy `char::is_alphanumeric()`; this allows non-ASCII alphanumerics.
- Sensitive auth request DTOs use `zeroize` where implemented.

## Database and Schema

The Diesel schema currently includes these tables:

- `users`
- `user_roles`
- `roles`
- `permissions`
- `role_permissions`
- `email_verification_tokens`
- `password_reset_tokens`
- `user_profile_picture_image_types`
- `user_profile_pictures`
- `posts`
- `comments`
- `post_votes`
- `comment_votes`
- `tags`
- `post_tags`
- `iso_country`
- `iso_country_subdivision`
- `iso_currency`
- `iso_language`
- `i18n_strings`
- `visitation_data`
- `photographs`
- `live_chat_messages`
- `live_chat_bans`
- `wasm_module`

Migrations also seed substantial ISO/country/language/currency data and define
role IDs. Do not infer the DB shape from domain structs alone; check
`src/schema.rs` and migrations together.

Diesel conventions:

- Domain structs derive combinations of `Queryable`, `QueryableByName`,
  `Selectable`, `Insertable`, `AsChangeset`, and `ToSchema`.
- Insert structs are named `New...` or `...Insertable`.
- Table column names generally keep verbose domain prefixes, such as
  `post_title`, `user_email`, `photograph_link`, `wasm_module_bundle_gz`.
- Many database functions are implemented directly in handlers rather than
  central service objects. Blog voting is an exception with service code under
  `src/domain/blog/service/`.

## Blog Domain

Important structs:

- `Post`: full DB row, includes rendered HTML content and `post_metadata`.
- `PostInfo`: reduced post row for list/cache use.
- `CachedPostInfo`: cache representation with tags.
- `PostInfoWithVote`: response representation with author badge and vote state.
- `Comment` and `CommentResponse`: comment DB row and decorated response.
- `Tag`, `PostTag`, `PostVote`, `CommentVote`: tag and vote tables.
- `VoteState`: serializes as integer values, not strings:
  - `0`: upvoted
  - `1`: downvoted
  - `2`: did not vote

Behavior and quirks:

- Submitted post markdown is rendered to HTML using `comrak`; the original
  markdown is saved inside `post_metadata.markdown_content`.
- Slugs are generated from titles with `util::string::generate_slug`.
- Post tags are trimmed, lowercased, deduplicated, and stored in `tags` plus
  `post_tags`.
- Creating or updating a post updates the post cache and Tantivy search index.
- Unpublished posts are removed from search.
- Post list reads from the cache first, then decorates with author info, profile
  picture, country flag, and the current user's vote state.
- `get_posts_from_cache(page, page_size, include_unpublished)` treats page size
  as at least 1 and sorts by `post_created_at` descending.
- Public post lists exclude unpublished posts unless the optional auth session is
  a superuser.

## Search

Search is implemented with Tantivy under `src/init/search/`.

- Index path defaults to `./data/search_index`.
- The post cache synchronization path keeps the search index coherent.
- Single-token title search uses `PhrasePrefixQuery`.
- Multi-token title search uses `QueryParser`.
- Tag searches use exact lowercased term queries.
- Multi-tag searches require all tags to match.

When changing blog write paths, check whether the Tantivy index should be
updated, removed, or rebuilt.

## Countries and i18n

Country/language/currency reference data is cached at startup:

- `CountryAndSubdivisionsTable`
- `IsoLanguageTable`
- `IsoCurrencyTable`

Country data exposes both list-shaped JSON snapshots and indexes by IDs or
alpha codes. `country_flag_for_country_code` and related helpers are used to
decorate blog and live chat actors.

i18n is backed by both files and DB:

- Source JSON files live in `i18n/ui/en-US.json` and `i18n/ui/ko-KR.json`.
- Required UI keys are enumerated in
  `src/domain/i18n/ui_text/keys.rs`.
- `sync_file_backed_ui_text_sources` validates the files and upserts them into
  `i18n_strings` using `Uuid::nil()` as the system user.
- `sync_i18n_data` loads all DB i18n strings into `I18nCache`.
- `I18nCache` indexes by country, subdivision, language, created/updated user,
  reference key, and time ranges.
- UI text bundle lookup falls back by country/language for required keys.

When adding a UI text key, update:

1. `REQUIRED_UI_TEXT_KEYS`
2. every JSON source bundle in `i18n/ui/`
3. any frontend usage expecting that key

## GeoIP and Visitor Board

GeoIP data is loaded from local files at startup:

- `./new_bundle_ipv4.db`
- `./new_bundle_ipv6.db`

The files are zstd-compressed bitcode bundles. They are decompressed into
interned string entries and stored as BTreeMaps keyed by IP range start. Lookup
uses the nearest preceding range start and then verifies the end bound.

Production request logging enqueues visitor data based on extracted client IP.
The visitor log buffer is periodically flushed by the job scheduler.

Client IP extraction lives in `src/util/extract/client_ip.rs`. The current TODO
mentions improving trusted proxy behavior, so be cautious with IP-related
security decisions.

## Photography and Image Processing

Photography models live under `src/domain/photography/`.

`PhotographContext` is a Postgres enum wrapper with values:

- `Photography`
- `Post`

`PhotographContext::from_str` accepts aliases such as `photography`,
`portfolio`, `gallery`, `post`, `posts`, `blog`, and `editor`.

Image processing:

- `process_uploaded_image` decodes uploaded bytes, optionally with a provided
  format fallback.
- Large images are resized according to `CyhdevImageType`.
- Output encoding is currently AVIF (`IMAGE_ENCODING_FORMAT`).
- Profile pictures max long edge: 400.
- Photographs max long edge: 6000.
- Thumbnails max long edge: 800.
- Demo thumbnails max long edge: 512.
- CPU-heavy processing runs in `spawn_blocking`.

The S3 bucket name is not centralized across all handlers. Check each handler
before changing upload/delete behavior.

## WASM Module Hosting

The `wasm_module` table stores metadata plus `wasm_module_bundle_gz`.

Accepted upload forms:

- Multipart bundle fields: `bundle_file`, `wasm_file`, or `wasm`.
- Bundle may be `.html`, `.html.gz`, `.wasm`, or `.wasm.gz`.
- Multipart thumbnail fields: `thumbnail` or `thumbnail_file`.
- Text fields: `title`/`wasm_module_title` and
  `description`/`wasm_module_description`.

Bundle behavior:

- Max bundle size is 50 MB.
- Bundles are normalized and stored gzipped at maximum compression.
- HTML bundles are detected by content type, file extension, or HTML-looking
  bytes.
- WASM bundles must have the `\0asm` magic bytes after decompression.
- Served bundles come from `ServerState.wasm_module_cache` when possible.
- `GET /api/wasm-modules/{wasm_module_id}/wasm` sets content type, long cache
  headers, permissive CORS, and `Content-Encoding: gzip` for cached gzipped
  bundles.

## Live Chat

Live chat has both HTTP history/stats endpoints and a WebSocket endpoint at
`/ws/live-chat`.

Constants:

- Default room: `main`.
- Initial messages sent on connect: 50.
- Max message chars: 300.
- Max frame bytes: 2 KB.
- Typing TTL: 4 seconds.
- Cache budget: 128 MB.
- Broadcast capacity: 1024.
- Abnormal message threshold: more than 10 messages per second per IP or user.

Actors:

- Logged-in users use `ChatActor::user` with session display name, country flag,
  and latest profile picture if available.
- Logged-out users use `ChatActor::guest`; guest names are deterministic from IP.

Protocols:

- The WebSocket offers a binary subprotocol named by
  `LIVE_CHAT_BINARY_PROTOCOL`.
- If selected, messages use the custom binary codec under
  `src/domain/live_chat/binary_codec*`.
- Otherwise JSON events are used.

Cache:

- Messages are keyed by UUID and separately indexed by a timeline key
  `(room_key, created_at_micros, message_id)`.
- Eviction uses an estimated byte budget and queue.
- Typing state, connection state, bans, and rate state are all in the cache.
- Bans are cached by user ID and IP. Expired bans are removed lazily.

Send-message flow:

1. Recheck ban state.
2. Record rate attempt; abnormal patterns persist a ban and close the socket.
3. Trim and validate message body.
4. Persist to DB.
5. Append to cache.
6. Clear typing state if needed.
7. Send ack to sender.
8. Broadcast message to subscribers.

## Background Jobs

`task_init` starts recurring Tokio tasks:

- Every hour at minute 30: invalidate expired sessions.
- Every hour at minute 0: purge non-verified users.
- Every second: update system stats.
- Every day at 06:30: compress old logs.
- Every minute: flush visitor logs.

Scheduler helpers live in `src/jobs/job_funcs/`.

These jobs run in-process. There is no distributed scheduler coordination here,
so multiple running instances may duplicate background work unless deployment
adds coordination outside the app.

## Utilities

Notable utility modules:

- `util/crypto`: Argon2 password hash/verify and random password generation.
- `util/email`: validation and password reset email templates.
- `util/extract`: client IP and host extraction.
- `util/geographic`: GeoIP bundle processing and lookup.
- `util/image`: upload image processing, EXIF helpers, DB image type mapping.
- `util/string`: username/password validation and slug generation.
- `util/system`: CPU/memory/process metrics with some unit tests.
- `util/time`: timestamp helpers and duration formatting.
- `util/wasm_bundle`: gzip normalization, detection, and content type sniffing.

Prefer these utilities over duplicating logic in handlers.

## OpenAPI Conventions

For each documented endpoint:

1. Add `#[utoipa::path(...)]` to the handler function.
2. Ensure request/response/domain structs derive `ToSchema` where needed.
3. Import the handler module and schema types in `src/docs.rs`.
4. Add the handler to `paths(...)`.
5. Add schema types to `components(schemas(...))`.
6. Register or reuse a tag in `tags(...)`.

OpenAPI does not discover routes from Axum automatically.

## Build, Docker, and Tooling

Useful commands:

- `cargo check`
- `cargo clippy`
- `cargo test`
- `cargo run`
- `diesel migration run`
- `docker compose up --build`

The local clippy configuration warns on `unwrap_used` and `expect_used`.
Existing code still contains some generated/build-time `expect` usage, but new
runtime code should avoid `unwrap` and `expect`.

Docker notes:

- `Dockerfile` uses `rust:<RUST_VERSION>-alpine`, builds release, compresses the
  binary with `upx`, then copies into a `scratch` final image.
- The build stage bind-mounts `src`, `fe`, `Cargo.toml`, and `Cargo.lock`.
- The final image expects GeoIP bundle files copied into `/bin/`.
- `compose.yaml` exposes host port `30737` to container port `30737`, but the
  Dockerfile exposes `443` and sets `HOST_PORT=443`; verify this before relying
  on compose as-is.

## Testing State

There is no broad integration test suite in this repository. Unit tests are
currently concentrated in:

- `src/util/system/get_memory_size.rs`
- `src/util/system/get_memory_usage.rs`
- `src/util/system/get_cpu_usage.rs`
- `src/domain/live_chat/binary_codec.rs`
- `src/domain/live_chat/guest_nickname.rs`

For changes touching handlers, DB behavior, or cache coherence, add focused
tests if practical and at least run `cargo check`. Many runtime paths require
environment variables, TLS files, database access, AWS credentials, and GeoIP
bundle files.

## Common Change Checklist

Adding a new API endpoint:

1. Add request/response DTOs if the payload is not trivial.
2. Implement a handler in the matching `src/handlers/<domain>/` module.
3. Use `HandlerResponse<impl IntoResponse>` and `http_resp`.
4. Use `CodeError` constants for failures.
5. Register the route in the correct access tier in `main_router.rs`.
6. Add middleware expectations through `Extension` arguments if auth context is
   needed.
7. Add OpenAPI attributes and update `src/docs.rs`.
8. Update caches or background jobs if the endpoint mutates cached data.

Adding a database-backed model:

1. Create a Diesel migration.
2. Regenerate or update `src/schema.rs` through Diesel tooling.
3. Add domain structs with the appropriate Diesel derives.
4. Add DTOs separately; avoid exposing sensitive DB fields by accident.
5. Decide whether startup cache synchronization is needed.
6. Add OpenAPI schema derives only for types that are safe to expose.

Changing auth or role behavior:

1. Check `RoleType` fixed UUID mappings.
2. Check both `auth_middleware` and `is_logged_in_middleware`.
3. Check cookie domain and secure settings for all deployment environments.
4. Consider session refresh and invalidation jobs.
5. Avoid assuming sessions survive process restarts.

Changing blog writes:

1. Preserve markdown-in-metadata behavior unless intentionally migrating.
2. Keep post cache, slug cache, order cache, and search index coherent.
3. Normalize tags the same way everywhere: trim, lowercase, dedupe.
4. Preserve `VoteState` serialized integer values for API compatibility.

Changing file-backed UI text:

1. Update required key list.
2. Update every locale JSON file.
3. Ensure startup sync still succeeds; missing keys are fatal to source parsing.
4. Consider the fallback country/language behavior.

Changing image upload flows:

1. Enforce max upload sizes before expensive processing.
2. Run decode/resize/encode in `spawn_blocking`.
3. Remember output is AVIF unless `IMAGE_ENCODING_FORMAT` changes.
4. Update DB image-type mapping if encoding behavior changes.

Changing live chat:

1. Keep JSON and binary protocol behavior aligned.
2. Respect max frame size, message length, typing TTL, and rate-ban behavior.
3. Keep cache append, DB persistence, and broadcast ordering coherent.
4. Update both HTTP history/stats and WebSocket event types if response shapes
   change.

## Known Sharp Edges

- `src/build_info.rs` is generated in the source tree and may appear modified
  after builds.
- API key middleware is present but commented out in `main_router.rs`.
- OpenAPI docs require manual registration and are not fully synchronized with
  all current routes.
- Sessions are in memory only.
- Swagger is mounted in all environments but protected only in `Prod`.
- Docker compose port mapping appears inconsistent with `HOST_PORT=443`.
- The app requires local GeoIP bundle files at startup.
- Startup requires many external dependencies: Postgres, SMTP config, AWS image
  credentials, TLS files, and file-backed i18n data.
- Visitor logging only enqueues in production.
- Some route naming is historical and not uniformly RESTful.
- The owner/superuser role is named `Younghyun`; do not replace it with a
  generic admin role without migrating role constants and DB seed data.
