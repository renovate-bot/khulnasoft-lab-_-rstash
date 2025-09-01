# Available Configuration Options

## file

```toml
# If specified, wait this long for the server to start up.
server_startup_timeout_ms = 10000

[dist]
# where to find the scheduler
scheduler_url = "http://1.2.3.4:10600"
# a set of prepackaged toolchains
toolchains = []
# the maximum size of the toolchain cache in bytes
toolchain_cache_size = 5368709120
cache_dir = "/home/user/.cache/rstash-dist-client"

[dist.auth]
type = "token"
token = "secrettoken"


#[cache.azure]
# does not work as it appears

[cache.disk]
dir = "/tmp/.cache/rstash"
size = 7516192768 # 7 GiBytes

# See the local docs on more explanations about this mode
[cache.disk.preprocessor_cache_mode]
# Whether to use the preprocessor cache mode
use_preprocessor_cache_mode = true
# Whether to use file times to check for changes
file_stat_matches = true
# Whether to also use ctime (file status change) time to check for changes
use_ctime_for_stat = true
# Whether to ignore `__TIME__` when caching
ignore_time_macros = false
# Whether to skip (meaning not cache, only hash) system headers
skip_system_headers = false
# Whether hash the current working directory
hash_working_directory = true

[cache.gcs]
# optional oauth url
oauth_url = "..."
# optional deprecated url
deprecated_url = "..."
rw_mode = "READ_ONLY"
# rw_mode = "READ_WRITE"
cred_path = "/psst/secret/cred"
bucket = "bucket"
key_prefix = "prefix"

[cache.gha]
url = "http://localhost"
token = "secret"
cache_to = "rstash-latest"
cache_from = "rstash-"

[cache.memcached]
# Deprecated alias for `endpoint`
# url = "127.0.0.1:11211"
endpoint = "tcp://127.0.0.1:11211"
# Username and password for authentication
username = "user"
password = "passwd"
# Entry expiration time in seconds. Default is 86400 (24 hours)
expiration = 3600
key_prefix = "/custom/prefix/if/need"

[cache.redis]
# Deprecated, use `endpoint` instead
url = "redis://user:passwd@1.2.3.4:6379/?db=1"
## Refer to the `opendal` documentation for more information about Redis endpoint
# Single-node endpoint. Mutually exclusive with `cluster_endpoints`
endpoint = "redis://127.0.0.1:6379"
# Multiple-node list of endpoints (cluster mode). Mutually exclusive with `endpoint`
cluster_endpoints = "redis://10.0.0.1:6379,redis://10.0.0.2:6379"
username = "user"
password = "passwd"
# Database number to use. Default is 0
db = 1
# Entry expiration time in seconds. Default is 0 (never expire)
expiration = 3600
key_prefix = "/custom/prefix/if/need"

[cache.s3]
bucket = "name"
endpoint = "s3-us-east-1.amazonaws.com"
use_ssl = true
key_prefix = "s3prefix"
server_side_encryption = false

[cache.webdav]
endpoint = "http://192.168.10.42:80/some/webdav.php"
key_prefix = "/custom/webdav/subfolder/if/need"
# Basic HTTP authentication credentials.
username = "alice"
password = "secret12"
# Mutually exclusive with username & password. Bearer token value
token = "token123"

[cache.oss]
bucket = "name"
endpoint = "oss-us-east-1.aliyuncs.com"
key_prefix = "ossprefix"
no_credentials = true
```

rstash looks for its configuration file at the path indicated by env variable `RSTASH_CONF`.

If no such env variable is set, rstash looks at default locations as below:
- Linux: `~/.config/rstash/config`
- macOS: `~/Library/Application Support/KhulnaSoft.rstash/config`
- Windows: `%APPDATA%\KhulnaSoft\rstash\config\config`

The latest `cache.XXX` entries may be found here: https://github.com/khulnasoft-lab/rstash/blob/ffe3070f77ef3301c8ff718316e4ab017ec83042/src/config.rs#L300.

## env

Whatever is set by a file based configuration, it is overruled by the env
configuration variables

### misc

* `RSTASH_ALLOW_CORE_DUMPS` to enable core dumps by the server
* `RSTASH_CONF` configuration file path
* `RSTASH_CACHED_CONF`
* `RSTASH_IDLE_TIMEOUT` how long the local daemon process waits for more client requests before exiting, in seconds. Set to `0` to run rstash permanently
* `RSTASH_STARTUP_NOTIFY` specify a path to a socket which will be used for server completion notification
* `RSTASH_MAX_FRAME_LENGTH` how much data can be transferred between client and server
* `RSTASH_NO_DAEMON` set to `1` to disable putting the server to the background
* `RSTASH_CACHE_MULTIARCH` to disable caching of multi architecture builds.
* `RSTASH_CACHE_ZSTD_LEVEL` to set zstd compression level of cache. the range is `1-22` and default is `3`.
  - For example, in `10`, it have about 0.9x size with about 1.6x time than default `3` (tested with compiling rstash code)
  - This option will only applied to newly compressed cache and don't affect existing cache.
  - If you want to be apply to all cache, you should reset cache and make new cache.

### cache configs

#### disk (local)

* `RSTASH_DIR` local on disk artifact cache directory
* `RSTASH_CACHE_SIZE` maximum size of the local on disk cache i.e. `2G` - default is 10G
* `RSTASH_DIRECT` enable/disable preprocessor caching (see [the local doc](Local.md))
* `RSTASH_LOCAL_RW_MODE` the mode that the cache will operate in (`READ_ONLY` or `READ_WRITE`)

#### s3 compatible

* `RSTASH_BUCKET` s3 bucket to be used
* `RSTASH_ENDPOINT` s3 endpoint
* `RSTASH_REGION` s3 region, required if using AWS S3
* `RSTASH_S3_USE_SSL` s3 endpoint requires TLS, set this to `true`
* `RSTASH_S3_KEY_PREFIX` s3 key prefix (optional)

The endpoint used then becomes `${RSTASH_BUCKET}.s3-{RSTASH_REGION}.amazonaws.com`.
If you are not using the default endpoint and `RSTASH_REGION` is undefined, it
will default to `us-east-1`.

#### cloudflare r2

* `RSTASH_BUCKET` is the name of your R2 bucket.
* `RSTASH_ENDPOINT` must follow the format of `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`. Note that the `https://` must be included. Your account ID can be found [here](https://developers.cloudflare.com/fundamentals/get-started/basic-tasks/find-account-and-zone-ids/).
* `RSTASH_REGION` should be set to `auto`.
* `RSTASH_S3_KEY_PREFIX` s3 key prefix (optional).

#### redis

* `RSTASH_REDIS` full redis url, including auth and access token/passwd (deprecated).
* `RSTASH_REDIS_ENDPOINT` redis url without auth and access token/passwd - single node configuration.
* `RSTASH_REDIS_CLUSTER_ENDPOINTS` redis cluster urls, separated by comma - shared cluster configuration.
* `RSTASH_REDIS_USERNAME` redis username (optional).
* `RSTASH_REDIS_PASSWORD` redis password (optional).
* `RSTASH_REDIS_DB` redis database (optional, default is 0).
* `RSTASH_REDIS_EXPIRATION` / `RSTASH_REDIS_TTL` ttl for redis cache, don't set for default behavior.
* `RSTASH_REDIS_KEY_PREFIX` key prefix (optional).

The full url appears then as `redis://user:passwd@1.2.3.4:6379/?db=1`.

#### memcached

* `RSTASH_MEMCACHED` is a deprecated alias for `RSTASH_MEMCACHED_ENDPOINT`.
* `RSTASH_MEMCACHED_ENDPOINT` memcached url.
* `RSTASH_MEMCACHED_USERNAME` memcached username (optional).
* `RSTASH_MEMCACHED_PASSWORD` memcached password (optional).
* `RSTASH_MEMCACHED_EXPIRATION` ttl for memcached cache, don't set for default behavior.
* `RSTASH_MEMCACHED_KEY_PREFIX` key prefix (optional).

#### gcs

* `RSTASH_GCS_BUCKET`
* `RSTASH_GCS_CREDENTIALS_URL`
* `RSTASH_GCS_KEY_PATH`
* `RSTASH_GCS_RW_MODE`

#### azure

* `RSTASH_AZURE_CONNECTION_STRING`

#### gha

* `RSTASH_GHA_CACHE_URL` / `ACTIONS_RESULTS_URL` GitHub Actions cache API URL
* `RSTASH_GHA_RUNTIME_TOKEN` / `ACTIONS_RUNTIME_TOKEN` GitHub Actions access token
* `RSTASH_GHA_CACHE_TO` cache key to write
* `RSTASH_GHA_CACHE_FROM` comma separated list of cache keys to read from

#### webdav

* `RSTASH_WEBDAV_ENDPOINT` a webdav service endpoint to store cache, such as `http://127.0.0.1:8080/my/webdav.php`.
* `RSTASH_WEBDAV_KEY_PREFIX` specify the key prefix (subfolder) of cache (optional).
* `RSTASH_WEBDAV_USERNAME` a username to authenticate with webdav service (optional).
* `RSTASH_WEBDAV_PASSWORD` a password to authenticate with webdav service (optional).
* `RSTASH_WEBDAV_TOKEN` a token to authenticate with webdav service (optional) - may be used instead of login & password.

#### OSS

* `RSTASH_OSS_BUCKET`
* `RSTASH_OSS_ENDPOINT`
* `RSTASH_OSS_KEY_PREFIX`
* `ALIBABA_CLOUD_ACCESS_KEY_ID`
* `ALIBABA_CLOUD_ACCESS_KEY_SECRET`
* `RSTASH_OSS_NO_CREDENTIALS`
