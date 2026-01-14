mod db;
mod models;

use anyhow::{Context, Result};
// use dotenv::dotenv;
use log::info;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::db::DbClient;
use crate::models::{StgContest, StgGame, StgOutcome, StgVenue};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    info!("Starting data loader");

    // Load environment variables
    dotenv::from_filename("../.env.development").ok();

    // Read and parse JSON file
    let file_path = Path::new("stg_records.json");
    let file = File::open(file_path).context(format!("Failed to open {}", file_path.display()))?;

    // Get file size for logging
    let file_size = file
        .metadata()
        .context("Failed to get file metadata")?
        .len();
    info!("File size: {} bytes", file_size);

    let mut reader = BufReader::new(file);

    // Read the first few bytes to check for BOM or other issues
    let mut header = [0u8; 4];
    let header_len = reader
        .read(&mut header)
        .context("Failed to read file header")?;
    info!("File header (hex): {:02x?}", &header[..header_len]);

    // Reset file position to start
    reader
        .seek(SeekFrom::Start(0))
        .context("Failed to seek to file start")?;

    // Read the entire file into a string
    let mut content = String::with_capacity(file_size as usize);
    let bytes_read = reader
        .read_to_string(&mut content)
        .context("Failed to read file content")?;

    // Verify we read the entire file
    if bytes_read != file_size as usize {
        info!(
            "Warning: Read {} bytes but file size is {} bytes",
            bytes_read, file_size
        );
    }

    // Check for common file corruption issues
    let content_len = content.len();
    info!("Content length: {} characters", content_len);
    info!("Number of newlines: {}", content.matches('\n').count());
    info!("Number of opening braces: {}", content.matches('{').count());
    info!("Number of closing braces: {}", content.matches('}').count());
    info!(
        "Number of opening brackets: {}",
        content.matches('[').count()
    );
    info!(
        "Number of closing brackets: {}",
        content.matches(']').count()
    );

    // Log the first and last few characters for debugging
    let preview_len = 100;
    info!(
        "First {} characters: {:?}",
        preview_len,
        &content[..content.len().min(preview_len)]
    );
    info!(
        "Last {} characters: {:?}",
        preview_len,
        &content[content.len().saturating_sub(preview_len)..]
    );

    // Validate JSON structure before parsing
    if !content.trim().starts_with('[') || !content.trim().ends_with(']') {
        info!("JSON content does not start with '[' or end with ']'");
        info!("First character: {:?}", content.chars().next());
        info!("Last character: {:?}", content.chars().last());
        return Err(anyhow::anyhow!("Invalid JSON structure: must be an array"));
    }

    // Try to parse the JSON with more detailed error reporting
    let contests: Vec<StgContest> = match serde_json::from_str(&content) {
        Ok(contests) => contests,
        Err(_e) => {
            // If parsing fails, try to parse as a generic Value first
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    info!("Successfully parsed as Value, examining structure:");
                    if let Some(array) = value.as_array() {
                        info!("Value is an array with {} elements", array.len());

                        // Try to parse just the first element with detailed error reporting
                        if let Some(first_element) = array.first() {
                            let element_str = serde_json::to_string(first_element)?;
                            info!("First element as string: {}", element_str);

                            // Try to parse each field individually to find the problematic one
                            let value: serde_json::Value = serde_json::from_str(&element_str)?;
                            if let Some(obj) = value.as_object() {
                                for (key, val) in obj {
                                    info!("Field {}: {:?}", key, val);
                                    match key.as_str() {
                                        "start" | "stop" => {
                                            info!("DateTime field {}: {}", key, val);
                                            if let Some(dt_str) = val.as_str() {
                                                match chrono::DateTime::parse_from_rfc3339(dt_str) {
                                                    Ok(_dt) => info!(
                                                        "Successfully parsed {} as DateTime",
                                                        key
                                                    ),
                                                    Err(e) => info!(
                                                        "Failed to parse {} as DateTime: {}",
                                                        key, e
                                                    ),
                                                }
                                            }
                                        }
                                        "venue" => {
                                            info!(
                                                "Venue field: {}",
                                                serde_json::to_string_pretty(val).unwrap()
                                            );
                                            match serde_json::from_value::<StgVenue>(val.clone()) {
                                                Ok(_) => info!("Successfully parsed venue"),
                                                Err(e) => info!("Failed to parse venue: {}", e),
                                            }
                                        }
                                        "games" => {
                                            info!(
                                                "Games field: {}",
                                                serde_json::to_string_pretty(val).unwrap()
                                            );
                                            match serde_json::from_value::<Vec<StgGame>>(
                                                val.clone(),
                                            ) {
                                                Ok(_) => info!("Successfully parsed games"),
                                                Err(e) => info!("Failed to parse games: {}", e),
                                            }
                                        }
                                        "outcome" => {
                                            info!(
                                                "Outcome field: {}",
                                                serde_json::to_string_pretty(val).unwrap()
                                            );
                                            match serde_json::from_value::<Vec<StgOutcome>>(
                                                val.clone(),
                                            ) {
                                                Ok(_) => info!("Successfully parsed outcome"),
                                                Err(e) => info!("Failed to parse outcome: {}", e),
                                            }
                                        }
                                        _ => info!("Other field {}: {}", key, val),
                                    }
                                }
                            }

                            // Try parsing the whole element
                            match serde_json::from_str::<StgContest>(&element_str) {
                                Ok(_) => info!("Successfully parsed first element as StgContest"),
                                Err(e) => {
                                    info!("Failed to parse first element as StgContest: {}", e);
                                    return Err(e).context("Failed to parse first element");
                                }
                            }
                        }

                        // If we get here, try parsing all elements
                        let mut contests = Vec::with_capacity(array.len());
                        for (i, element) in array.iter().enumerate() {
                            let element_str = serde_json::to_string(element)?;
                            match serde_json::from_str::<StgContest>(&element_str) {
                                Ok(contest) => contests.push(contest),
                                Err(e) => {
                                    info!("Failed to convert element {}: {}", i, e);
                                    info!("Element as string: {}", element_str);
                                    return Err(e)
                                        .context(format!("Failed to convert element {}", i));
                                }
                            }
                        }
                        contests
                    } else {
                        info!(
                            "Value is not an array: {}",
                            serde_json::to_string_pretty(&value).unwrap()
                        );
                        return Err(anyhow::anyhow!("Expected JSON array"));
                    }
                }
                Err(e) => {
                    // If even Value parsing fails, show the error context
                    let line = e.line();
                    let col = e.column();
                    let start = content
                        .lines()
                        .take(line.saturating_sub(2))
                        .map(|l| l.len() + 1)
                        .sum::<usize>();
                    let end = content
                        .lines()
                        .take(line + 2)
                        .map(|l| l.len() + 1)
                        .sum::<usize>();
                    info!("Error at line {}, column {}", line, col);
                    info!(
                        "Error context (lines {} to {}):",
                        line.saturating_sub(2),
                        line + 2
                    );
                    info!("{}", &content[start..end]);

                    // Show the exact position of the error
                    if let Some(error_line) = content.lines().nth(line - 1) {
                        info!("Error line: {}", error_line);
                        info!("Error position: {}^", " ".repeat(col - 1));

                        // Show the actual bytes around the error position
                        let line_start = content
                            .lines()
                            .take(line - 1)
                            .map(|l| l.len() + 1)
                            .sum::<usize>();
                        let error_pos = line_start + col - 1;
                        let context_start = error_pos.saturating_sub(10);
                        let context_end = (error_pos + 10).min(content.len());
                        info!(
                            "Bytes around error: {:02x?}",
                            content.as_bytes()[context_start..context_end].to_vec()
                        );
                    }
                    return Err(e).context("Failed to parse JSON");
                }
            }
        }
    };

    info!("Loaded {} contests", contests.len());

    // Create database client
    let mut db = DbClient::new().await?;
    info!("Connected to database");

    // Load records into database
    db.load_records(contests).await?;
    info!("Successfully loaded all records");

    Ok(())
}
