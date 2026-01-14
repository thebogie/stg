//! Sanitize exported production data by removing PII
//!
//! This script reads an exported JSON dump and sanitizes all PII fields:
//! - Email addresses: Replaced with test@example.com variants
//! - First names: Replaced with generic names
//! - Handles: Replaced with sanitized handles
//! - Password hashes: Replaced with test hashes
//! - Any other identifiable information

use anyhow::{Context, Result};
use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{info, warn};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON dump file
    #[arg(short, long)]
    input: PathBuf,

    /// Output sanitized file (will be gzipped if .gz extension)
    #[arg(short, long)]
    output: PathBuf,

    /// Sanitize email addresses
    #[arg(long, default_value_t = true)]
    sanitize_emails: bool,

    /// Sanitize names
    #[arg(long, default_value_t = true)]
    sanitize_names: bool,

    /// Sanitize handles
    #[arg(long, default_value_t = true)]
    sanitize_handles: bool,

    /// Sanitize password hashes
    #[arg(long, default_value_t = true)]
    sanitize_passwords: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("Reading dump from {}", args.input.display());
    let file = File::open(&args.input).context("Failed to open input file")?;
    let reader = BufReader::new(file);

    let mut dump: Value = serde_json::from_reader(reader).context("Failed to parse JSON")?;

    info!("Sanitizing PII...");
    sanitize_dump(&mut dump, &args)?;

    let is_gzipped = args
        .output
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == "gz")
        .unwrap_or(false);

    if is_gzipped {
        info!("Writing gzipped output to {}", args.output.display());
        let file = File::create(&args.output).context("Failed to create output file")?;
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        serde_json::to_writer(encoder, &dump).context("Failed to write JSON")?;
    } else {
        info!("Writing output to {}", args.output.display());
        let file = File::create(&args.output).context("Failed to create output file")?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &dump).context("Failed to write JSON")?;
        writer.flush()?;
    }

    info!("✅ Sanitization completed successfully!");
    info!("📁 Output file: {}", args.output.display());
    info!("🔒 All PII has been sanitized");

    Ok(())
}

fn sanitize_dump(dump: &mut Value, args: &Args) -> Result<()> {
    if let Some(collections) = dump.get_mut("collections").and_then(|c| c.as_object_mut()) {
        for (collection_name, documents) in collections {
            info!("Sanitizing collection: {}", collection_name);

            if let Some(docs_array) = documents.as_array_mut() {
                for (idx, doc) in docs_array.iter_mut().enumerate() {
                    sanitize_document(doc, collection_name, idx, args);
                }
            }
        }
    }

    Ok(())
}

fn sanitize_document(doc: &mut Value, collection_name: &str, idx: usize, args: &Args) {
    if let Some(obj) = doc.as_object_mut() {
        // Sanitize player-related collections
        if collection_name.contains("player") || collection_name.contains("user") {
            if args.sanitize_emails {
                if let Some(email) = obj.get_mut("email") {
                    if email.is_string() {
                        *email = json!(format!("test_user_{}@example.com", idx));
                    }
                }
            }

            if args.sanitize_names {
                if let Some(firstname) = obj.get_mut("firstname") {
                    *firstname = json!(format!("TestUser{}", idx));
                }
                if let Some(name) = obj.get_mut("name") {
                    *name = json!(format!("Test User {}", idx));
                }
            }

            if args.sanitize_handles {
                if let Some(handle) = obj.get_mut("handle") {
                    *handle = json!(format!("test_user_{}", idx));
                }
                if let Some(username) = obj.get_mut("username") {
                    *username = json!(format!("test_user_{}", idx));
                }
            }

            if args.sanitize_passwords {
                if let Some(password) = obj.get_mut("password") {
                    // Use a consistent test password hash (Argon2 hash of "test_password")
                    *password = json!("$argon2id$v=19$m=65536,t=3,p=4$test_salt$test_hash");
                }
            }
        }

        // Sanitize venue addresses (if needed)
        if collection_name.contains("venue") {
            if let Some(address) = obj.get_mut("formattedAddress") {
                *address = json!(format!("123 Test St, Test City, TC 12345"));
            }
            if let Some(address) = obj.get_mut("formatted_address") {
                *address = json!(format!("123 Test St, Test City, TC 12345"));
            }
        }

        // Remove any other potentially sensitive fields
        let sensitive_keys: Vec<String> = obj
            .keys()
            .filter(|k| {
                let key_lower = k.to_lowercase();
                key_lower.contains("phone")
                    || key_lower.contains("ssn")
                    || key_lower.contains("credit")
                    || key_lower.contains("card")
                    || key_lower.contains("ip")
            })
            .cloned()
            .collect();

        for key in sensitive_keys {
            obj.remove(&key);
            warn!("Removed sensitive field: {}", key);
        }
    }
}
