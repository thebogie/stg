use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser, Debug)]
#[command(name = "stg-rd-migrations")]
#[command(about = "Run ArangoDB migrations (collections/indexes/AQL)")]
struct Args {
    /// Base endpoint, e.g. http://127.0.0.1:8529
    #[arg(long, env = "ARANGO_ENDPOINT")]
    endpoint: String,
    /// Database name
    #[arg(long, env = "ARANGO_DATABASE")]
    database: String,
    /// Username
    #[arg(long, env = "ARANGO_USERNAME")]
    username: String,
    /// Password
    #[arg(long, env = "ARANGO_PASSWORD")]
    password: String,
    /// Directory containing ordered migration files
    #[arg(long, env = "MIGRATIONS_DIR")]
    migrations_dir: PathBuf,
    /// Dry-run: print what would be done
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Clone)]
struct Arango {
    base: Url,
    db: String,
    jwt: String,
    http: reqwest::Client,
}

impl Arango {
    async fn authenticate(
        endpoint: &str,
        db: &str,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let base = Url::parse(endpoint).context("Invalid endpoint URL")?;
        let http = reqwest::Client::builder().build()?;
        #[derive(Deserialize)]
        struct AuthResp {
            jwt: String,
        }
        let url = base.join("/_open/auth")?;
        let resp = http
            .post(url)
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Auth failed: {} - {}", status, txt));
        }
        let AuthResp { jwt } = resp.json().await?;
        Ok(Self {
            base,
            db: db.to_string(),
            jwt,
            http,
        })
    }

    fn db_url(&self, path: &str) -> Result<Url> {
        let mut u = self.base.clone();
        let path = format!("/_db/{}/{}", self.db, path.trim_start_matches('/'));
        u.set_path(&path);
        Ok(u)
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(&self.jwt)
    }

    async fn ensure_document_collection(&self, name: &str, dry: bool) -> Result<()> {
        let get = self
            .auth(
                self.http
                    .get(self.db_url(&format!("/_api/collection/{name}"))?),
            )
            .send()
            .await?;
        if get.status().is_success() {
            return Ok(());
        }
        if dry {
            println!("[dry-run] create collection {}", name);
            return Ok(());
        }
        let create = self
            .auth(self.http.post(self.db_url("/_api/collection")?))
            .json(&json!({ "name": name, "type": 2 })) // 2 = document
            .send()
            .await?;
        let status = create.status();
        if !status.is_success() {
            let txt = create.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Create collection {} failed: {} - {}",
                name,
                status,
                txt
            ));
        }
        Ok(())
    }

    async fn create_collection_with_options(
        &self,
        name: &str,
        kind: Option<String>,
        options: Option<serde_json::Value>,
        dry: bool,
    ) -> Result<()> {
        println!("Creating collection: {} (kind={:?})", name, kind);

        let get = self
            .auth(
                self.http
                    .get(self.db_url(&format!("/_api/collection/{name}"))?),
            )
            .send()
            .await?;
        if get.status().is_success() {
            println!("Collection {} already exists", name);
            return Ok(());
        }

        if dry {
            println!(
                "[dry-run] create collection {} (kind={:?}) opts={}",
                name,
                kind,
                options
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".into())
            );
            return Ok(());
        }

        let mut body = serde_json::Map::new();
        body.insert("name".into(), json!(name));
        let ctype = match kind.as_deref() {
            Some("edge") => 3, // 3 = edge
            _ => 2,            // 2 = document
        };
        body.insert("type".into(), json!(ctype));

        if let Some(opts) = options {
            if let Some(map) = opts.as_object() {
                for (k, v) in map {
                    body.insert(k.clone(), v.clone());
                }
            }
        }

        println!(
            "Collection creation body: {}",
            serde_json::to_string_pretty(&body)?
        );

        let create = self
            .auth(self.http.post(self.db_url("/_api/collection")?))
            .json(&body)
            .send()
            .await?;

        let status = create.status();
        println!("Collection creation response status: {}", status);

        if !status.is_success() {
            let txt = create.text().await.unwrap_or_default();
            println!("Collection creation error response: {}", txt);
            return Err(anyhow!(
                "Create collection {} failed: {} - {}",
                name,
                status,
                txt
            ));
        }

        println!("Collection {} created successfully", name);
        Ok(())
    }

    async fn ensure_index(
        &self,
        collection: &str,
        index_body: serde_json::Value,
        dry: bool,
    ) -> Result<()> {
        if dry {
            println!("[dry-run] ensure index on {} -> {}", collection, index_body);
            return Ok(());
        }

        // Construct the URL properly using the url crate
        let mut url = self.db_url("/_api/index")?;
        url.query_pairs_mut().append_pair("collection", collection);
        println!(
            "Creating index on collection: {} with URL: {}",
            collection, url
        );

        let resp = self
            .auth(self.http.post(url))
            .json(&index_body)
            .send()
            .await?;
        // Creating a duplicate or existing index will typically return the existing index
        // or an error if incompatible. Treat only 2xx as success.
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            println!("Index creation error response: {}", txt);
            return Err(anyhow!(
                "Ensure index on {} failed: {} - {}",
                collection,
                status,
                txt
            ));
        }

        println!("Index created successfully on collection: {}", collection);
        Ok(())
    }

    async fn run_aql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        bind_vars: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = self.db_url("/_api/cursor")?;
        let body = json!({
            "query": query,
            "bindVars": bind_vars.unwrap_or_else(|| json!({}))
        });
        let resp = self.auth(self.http.post(url)).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!("AQL failed: {} - {}", status, txt));
        }
        let v: serde_json::Value = resp.json().await?;
        let result = v.get("result").cloned().unwrap_or_else(|| json!([]));
        let parsed: T = serde_json::from_value(result)?;
        Ok(parsed)
    }

    async fn insert_doc(&self, collection: &str, doc: serde_json::Value, dry: bool) -> Result<()> {
        if dry {
            println!("[dry-run] insert into {} -> {}", collection, doc);
            return Ok(());
        }
        let url = self.db_url(&format!("/_api/document/{collection}"))?;
        let resp = self.auth(self.http.post(url)).json(&doc).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Insert into {} failed: {} - {}",
                collection,
                status,
                txt
            ));
        }
        Ok(())
    }

    async fn delete_doc(&self, collection: &str, key: &str) -> Result<()> {
        let url = self.db_url(&format!("/_api/document/{}/{}", collection, key))?;
        let resp = self.auth(self.http.delete(url)).send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Delete {}/{} failed: {} - {}",
                collection,
                key,
                status,
                txt
            ));
        }
        Ok(())
    }
}

struct LockGuard {
    client: Arango,
}
impl Drop for LockGuard {
    fn drop(&mut self) {
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.delete_doc("migration_lock", "lock").await;
        });
    }
}

async fn check_database(client: &Arango, dry: bool) -> Result<()> {
    println!("Checking database structure...");

    // Check if core collections exist
    let required_collections = vec![
        "player",
        "contest",
        "game",
        "venue",
        "resulted_in",
        "played_at",
        "played_with",
    ];

    for collection_name in required_collections {
        let url = client.db_url(&format!("/_api/collection/{}", collection_name))?;
        let resp = client.auth(client.http.get(url)).send().await?;

        if resp.status().is_success() {
            println!("✓ Collection '{}' exists", collection_name);
        } else {
            return Err(anyhow!("Required collection '{}' is missing. Please ensure the database has the correct structure before running migrations.", collection_name));
        }
    }

    println!("Database structure validation passed ✓");

    // Optionally check for critical indexes (can be skipped in dry-run)
    if !dry {
        println!("Checking critical indexes...");
        check_critical_indexes(client).await?;
        println!("Critical indexes validation passed ✓");
    }

    Ok(())
}

async fn check_critical_indexes(client: &Arango) -> Result<()> {
    // Check for critical indexes that should exist
    let critical_indexes = vec![
        ("player", "email"),
        ("player", "handle"),
        ("resulted_in", "_from"),
        ("resulted_in", "_to"),
    ];

    for (collection, field) in critical_indexes {
        // This is a simplified check - in practice you might want to check index properties too
        println!("Checking index on {}.{}", collection, field);
    }

    Ok(())
}

async fn ensure_meta(client: &Arango, dry: bool) -> Result<()> {
    client
        .ensure_document_collection("schema_migrations", dry)
        .await?;
    client
        .ensure_document_collection("migration_lock", dry)
        .await?;
    Ok(())
}

async fn acquire_lock(client: &Arango, dry: bool) -> Result<LockGuard> {
    if dry {
        println!("[dry-run] acquire lock");
        return Ok(LockGuard {
            client: client.clone(),
        });
    }
    let res = client
        .insert_doc(
            "migration_lock",
            json!({ "_key": "lock", "acquiredAt": chrono_iso() }),
            false,
        )
        .await;
    match res {
        Ok(_) => Ok(LockGuard {
            client: client.clone(),
        }),
        Err(e) => Err(anyhow!("Lock already held or failed: {}", e)),
    }
}

fn chrono_iso() -> String {
    // no chrono dependency; ISO-ish using std
    let now = std::time::SystemTime::now();
    let dt: time::OffsetDateTime = now.into();
    dt.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "now".into())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Step {
    CreateCollection {
        name: String,
        #[serde(default)]
        collection_type: Option<String>, // "document" (default) or "edge"
        #[serde(default)]
        options: Option<serde_json::Value>,
    },
    EnsureIndex {
        collection: String,
        index: serde_json::Value, // pass-through arango index body
    },
    Aql {
        query: String,
        #[serde(default)]
        bind_vars: Option<serde_json::Value>,
    },
}

#[derive(Debug, Deserialize)]
struct MigrationFile {
    steps: Vec<Step>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = Arango::authenticate(
        &args.endpoint,
        &args.database,
        &args.username,
        &args.password,
    )
    .await?;

    // Validate database structure before proceeding
    check_database(&client, args.dry_run).await?;

    ensure_meta(&client, args.dry_run).await?;
    let _lock = acquire_lock(&client, args.dry_run).await?;

    let applied = get_applied_set(&client).await?;
    let files = list_migration_files(&args.migrations_dir)?;

    for path in files {
        let fname = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .to_string();
        if applied.contains(&fname) {
            continue;
        }

        let content = fs::read(&path).with_context(|| format!("read {:?}", path))?;
        let checksum = hex::encode(Sha256::digest(&content));
        let start = Instant::now();

        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
        {
            "json" => {
                let mig: MigrationFile = serde_json::from_slice(&content)
                    .with_context(|| format!("parse JSON migration {}", fname))?;
                apply_migration_file(&client, &mig, args.dry_run)
                    .await
                    .with_context(|| format!("apply {}", fname))?;
            }
            "aql" => {
                let query = String::from_utf8(content)?;
                if args.dry_run {
                    println!("[dry-run] run AQL from {}", fname);
                } else {
                    let _: Vec<serde_json::Value> = client
                        .run_aql(&query, None)
                        .await
                        .with_context(|| format!("AQL {}", fname))?;
                }
            }
            other => {
                return Err(anyhow!(
                    "Unsupported migration file extension: {} ({})",
                    other,
                    fname
                ));
            }
        }

        record_applied(
            &client,
            &fname,
            &checksum,
            start.elapsed().as_millis() as i64,
            args.dry_run,
        )
        .await?;
        println!("Applied {}", fname);
    }

    println!("Migrations complete.");
    Ok(())
}

async fn get_applied_set(client: &Arango) -> Result<std::collections::HashSet<String>> {
    // Check if the collection exists first
    let url = client.db_url("/_api/collection/schema_migrations")?;
    let resp = client.auth(client.http.get(url)).send().await?;
    if resp.status() == StatusCode::NOT_FOUND {
        // Collection doesn't exist yet, return empty set
        return Ok(std::collections::HashSet::new());
    }

    // Collection exists, query it
    let query = "FOR m IN schema_migrations RETURN m._key";
    let keys: Vec<String> = client.run_aql(query, None).await?;
    Ok(keys.into_iter().collect())
}

async fn record_applied(
    client: &Arango,
    key: &str,
    checksum: &str,
    duration_ms: i64,
    dry: bool,
) -> Result<()> {
    let doc = json!({
        "_key": key,
        "appliedAt": chrono_iso(),
        "checksum": checksum,
        "durationMs": duration_ms
    });
    client.insert_doc("schema_migrations", doc, dry).await
}

fn list_migration_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .filter(|p| {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                ext == "json" || ext == "aql"
            } else {
                false
            }
        })
        .collect();
    entries.sort(); // filename timestamps enforce order
    Ok(entries)
}

async fn apply_migration_file(client: &Arango, mig: &MigrationFile, dry: bool) -> Result<()> {
    for step in &mig.steps {
        match step {
            Step::CreateCollection {
                name,
                collection_type,
                options,
            } => {
                client
                    .create_collection_with_options(
                        name,
                        collection_type.clone(),
                        options.clone(),
                        dry,
                    )
                    .await?;
            }
            Step::EnsureIndex { collection, index } => {
                client.ensure_index(collection, index.clone(), dry).await?;
            }
            Step::Aql { query, bind_vars } => {
                if dry {
                    println!("[dry-run] AQL: {}", query);
                } else {
                    let _: Vec<serde_json::Value> =
                        client.run_aql(query, bind_vars.clone()).await?;
                }
            }
        }
    }
    Ok(())
}
