#![cfg(feature = "db_integration")]

#[cfg(test)]
mod database_integration_tests {
    use super::*;
    use anyhow::Result;
    use arangors::{Connection, Database, client::ClientExt};
    use std::time::Duration;
    use tokio::time::sleep;
    use testing::TestEnvironment;

    #[tokio::test]
    async fn test_database_connection() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
        
        let db = conn.database("_system").await
            .map_err(|e| anyhow::anyhow!("Failed to access system database: {}", e))?;
        
        // Verify we can query the database
        let db_list = conn.list_databases().await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;
        assert!(!db_list.is_empty(), "Should have at least one database");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_database_creation_and_deletion() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_db_integration";
        
        // Create test database
        let db = conn.create_database(test_db_name).await
            .map_err(|e| anyhow::anyhow!("Failed to create test database: {}", e))?;
        
        // Verify database exists
        let db_list = conn.list_databases().await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;
        assert!(db_list.contains(&test_db_name.to_string()), 
            "Database {} should exist in list: {:?}", test_db_name, db_list);
        
        // Delete test database
        conn.drop_database(test_db_name).await
            .map_err(|e| anyhow::anyhow!("Failed to delete test database: {}", e))?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_collection_operations() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_collection_db";
        
        // Create test database
        let db = conn.create_database(test_db_name).await?;
        
        // Create test collection
        let collection_name = "test_collection";
        db.create_collection(collection_name).await
            .map_err(|e| anyhow::anyhow!("Failed to create collection: {}", e))?;
        
        // Verify collection exists
        let collections = db.list_collections().await
            .map_err(|e| anyhow::anyhow!("Failed to list collections: {}", e))?;
        assert!(collections.iter().any(|c| c.name == collection_name),
            "Collection {} should exist in list: {:?}", collection_name, 
            collections.iter().map(|c| &c.name).collect::<Vec<_>>());
        
        // Delete collection
        db.drop_collection(collection_name).await
            .map_err(|e| anyhow::anyhow!("Failed to delete collection: {}", e))?;
        
        // Cleanup - delete test database
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_operations() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_document_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await?;
        let collection = db.create_collection("test_docs").await?;
        
        // Test document insertion
        let test_doc = serde_json::json!({
            "name": "Test Document",
            "value": 42,
            "active": true
        });
        
        let inserted_doc = collection.create_document(test_doc.clone()).await
            .map_err(|e| anyhow::anyhow!("Failed to insert document: {}", e))?;
        
        assert!(inserted_doc._id.is_some(), "Inserted document should have an ID");
        
        // Test document retrieval
        let doc_id = inserted_doc._id.unwrap();
        let retrieved_doc = collection.document(&doc_id).await
            .map_err(|e| anyhow::anyhow!("Failed to retrieve document: {}", e))?;
        
        assert_eq!(retrieved_doc["name"], "Test Document", "Document name mismatch");
        assert_eq!(retrieved_doc["value"], 42, "Document value mismatch");
        assert_eq!(retrieved_doc["active"], true, "Document active flag mismatch");
        
        // Test document update
        let update_doc = serde_json::json!({
            "name": "Updated Document",
            "value": 100,
            "active": false
        });
        
        collection.update_document(&doc_id, update_doc).await
            .map_err(|e| anyhow::anyhow!("Failed to update document: {}", e))?;
        
        // Verify update
        let updated_doc = collection.document(&doc_id).await?;
        assert_eq!(updated_doc["name"], "Updated Document", "Updated document name mismatch");
        assert_eq!(updated_doc["value"], 100, "Updated document value mismatch");
        assert_eq!(updated_doc["active"], false, "Updated document active flag mismatch");
        
        // Test document deletion
        collection.remove_document(&doc_id).await
            .map_err(|e| anyhow::anyhow!("Failed to delete document: {}", e))?;
        
        // Verify deletion
        let deleted_doc = collection.document(&doc_id).await;
        assert!(deleted_doc.is_err(), "Document should not exist after deletion");
        
        // Cleanup
        db.drop_collection("test_docs").await?;
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_query_operations() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_query_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await?;
        let collection = db.create_collection("test_query").await?;
        
        // Insert test documents
        let test_docs = vec![
            serde_json::json!({"name": "Doc 1", "value": 10, "category": "A"}),
            serde_json::json!({"name": "Doc 2", "value": 20, "category": "B"}),
            serde_json::json!({"name": "Doc 3", "value": 30, "category": "A"}),
            serde_json::json!({"name": "Doc 4", "value": 40, "category": "C"}),
        ];
        
        for doc in test_docs {
            collection.create_document(doc).await?;
        }
        
        // Test simple query
        let query = "FOR doc IN test_query RETURN doc";
        let docs: Vec<serde_json::Value> = db.aql_query(query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute AQL query: {}", e))?;
        assert_eq!(docs.len(), 4, "Should have 4 documents");
        
        // Test filtered query
        let filtered_query = "FOR doc IN test_query FILTER doc.category == 'A' RETURN doc";
        let filtered_docs: Vec<serde_json::Value> = db.aql_query(filtered_query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute filtered AQL query: {}", e))?;
        assert_eq!(filtered_docs.len(), 2, "Should have 2 documents with category A");
        
        // Test sorted query
        let sorted_query = "FOR doc IN test_query SORT doc.value ASC RETURN doc";
        let sorted_docs: Vec<serde_json::Value> = db.aql_query(sorted_query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute sorted AQL query: {}", e))?;
        assert_eq!(sorted_docs.len(), 4, "Should have 4 documents");
        assert_eq!(sorted_docs[0]["value"], 10, "First document should have value 10");
        assert_eq!(sorted_docs[3]["value"], 40, "Last document should have value 40");
        
        // Test aggregated query
        let aggregate_query = "FOR doc IN test_query COLLECT category = doc.category WITH COUNT INTO count RETURN {category, count}";
        let aggregated: Vec<serde_json::Value> = db.aql_query(aggregate_query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute aggregate AQL query: {}", e))?;
        assert_eq!(aggregated.len(), 3, "Should have 3 categories");
        
        // Cleanup
        db.drop_collection("test_query").await?;
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_operations() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_transaction_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await?;
        db.create_collection("test_transaction").await?;
        
        // Test transaction with multiple operations
        let transaction_query = r#"
            LET doc1 = INSERT {name: "Transaction Doc 1", value: 100} INTO test_transaction
            LET doc2 = INSERT {name: "Transaction Doc 2", value: 200} INTO test_transaction
            LET doc3 = INSERT {name: "Transaction Doc 3", value: 300} INTO test_transaction
            RETURN {doc1, doc2, doc3}
        "#;
        
        db.aql_query(transaction_query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute transaction: {}", e))?;
        
        // Verify all documents were inserted
        let count_query = "FOR doc IN test_transaction COLLECT WITH COUNT INTO count RETURN count";
        let count: Vec<u64> = db.aql_query(count_query).await
            .map_err(|e| anyhow::anyhow!("Failed to count documents: {}", e))?;
        assert_eq!(count[0], 3, "Should have inserted 3 documents");
        
        // Cleanup
        db.drop_collection("test_transaction").await?;
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_index_operations() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_index_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await?;
        let collection = db.create_collection("test_index").await?;
        
        // Create index
        collection.create_index(&["name"]).await
            .map_err(|e| anyhow::anyhow!("Failed to create index: {}", e))?;
        
        // Verify index exists
        let indexes = collection.indexes().await
            .map_err(|e| anyhow::anyhow!("Failed to list indexes: {}", e))?;
        assert!(indexes.iter().any(|idx| idx.fields.contains(&"name".to_string())),
            "Index on 'name' should exist");
        
        // Test query performance with index
        let test_docs: Vec<serde_json::Value> = (0..1000)
            .map(|i| serde_json::json!({
                "name": format!("Doc {}", i),
                "value": i,
                "category": format!("Cat {}", i % 10)
            }))
            .collect();
        
        // Insert documents
        for doc in test_docs {
            collection.create_document(doc).await?;
        }
        
        // Test indexed query performance
        let start_time = std::time::Instant::now();
        let query = "FOR doc IN test_index FILTER doc.name == 'Doc 500' RETURN doc";
        let docs: Vec<serde_json::Value> = db.aql_query(query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute indexed query: {}", e))?;
        let query_time = start_time.elapsed();
        
        assert!(query_time.as_millis() < 1000, 
            "Indexed query should be fast, took {}ms", query_time.as_millis());
        assert_eq!(docs.len(), 1, "Should find exactly one document");
        assert_eq!(docs[0]["name"], "Doc 500", "Should find Doc 500");
        
        // Cleanup
        db.drop_collection("test_index").await?;
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pooling() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let arangodb_url = env.arangodb_url().to_string();
        let mut handles = Vec::new();
        
        // Test multiple concurrent connections
        for i in 0..10 {
            let url = arangodb_url.clone();
            let handle = tokio::spawn(async move {
                let conn = Connection::establish_basic_auth(&url, "root", "test_password").await
                    .map_err(|e| format!("Failed to establish connection {}: {}", i, e))?;
                
                let db = conn.database("_system").await
                    .map_err(|e| format!("Failed to access system database from connection {}: {}", i, e))?;
                
                // Simulate some work
                sleep(Duration::from_millis(100)).await;
                
                Ok::<(), String>(())
            });
            handles.push(handle);
        }
        
        // Wait for all connections to complete
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await
                .map_err(|e| anyhow::anyhow!("Connection task {} panicked: {:?}", i, e))?;
            result.map_err(|e| anyhow::anyhow!("Connection operation {} failed: {}", i, e))?;
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_database_performance() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        let test_db_name = "test_performance_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await?;
        let collection = db.create_collection("test_performance").await?;
        
        // Test bulk insert performance - use smaller dataset for faster tests
        let start_time = std::time::Instant::now();
        
        let test_docs: Vec<serde_json::Value> = (0..1000)
            .map(|i| serde_json::json!({
                "id": i,
                "name": format!("Performance Doc {}", i),
                "value": i * 2,
                "timestamp": chrono::Utc::now().timestamp()
            }))
            .collect();
        
        // Insert documents in batches
        let batch_size = 100;
        for batch in test_docs.chunks(batch_size) {
            for doc in batch {
                collection.create_document(doc.clone()).await?;
            }
        }
        
        let insert_time = start_time.elapsed();
        assert!(insert_time.as_secs() < 30, 
            "Bulk insert should complete within 30 seconds, took {}s", insert_time.as_secs());
        
        // Test query performance
        let start_time = std::time::Instant::now();
        let query = "FOR doc IN test_performance FILTER doc.value > 500 LIMIT 100 RETURN doc";
        let docs: Vec<serde_json::Value> = db.aql_query(query).await
            .map_err(|e| anyhow::anyhow!("Failed to execute performance query: {}", e))?;
        let query_time = start_time.elapsed();
        
        assert!(query_time.as_millis() < 5000, 
            "Query should complete within 5 seconds, took {}ms", query_time.as_millis());
        assert_eq!(docs.len(), 100, "Should return 100 documents");
        
        // Cleanup
        db.drop_collection("test_performance").await?;
        conn.drop_database(test_db_name).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        // Test invalid database access
        let invalid_db = conn.database("nonexistent_database").await;
        assert!(invalid_db.is_err(), "Should fail to access non-existent database");
        
        // Test invalid AQL query
        let db = conn.database("_system").await?;
        let invalid_query = "INVALID AQL SYNTAX";
        let result = db.aql_query(invalid_query).await;
        assert!(result.is_err(), "Should fail to execute invalid AQL query");
        
        // Test invalid collection access
        let invalid_collection = db.collection("nonexistent_collection").await;
        assert!(invalid_collection.is_err(), "Should fail to access non-existent collection");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_database_cleanup() -> Result<()> {
        let env = TestEnvironment::new().await?;
        env.wait_for_ready().await?;

        let conn = Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await?;
        
        // List all databases
        let db_list = conn.list_databases().await?;
        
        // Clean up test databases
        for db_name in &db_list {
            if db_name.starts_with("test_") {
                let _ = conn.drop_database(db_name).await;
            }
        }
        
        // Verify cleanup
        let db_list_after = conn.list_databases().await?;
        let test_dbs: Vec<&String> = db_list_after.iter().filter(|name| name.starts_with("test_")).collect();
        assert_eq!(test_dbs.len(), 0, "All test databases should be cleaned up");
        
        Ok(())
    }
}
