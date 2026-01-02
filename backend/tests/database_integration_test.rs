#![cfg(feature = "db_integration")]

#[cfg(test)]
mod database_integration_tests {
    use super::*;
    use arangors::{Connection, Database, client::ClientExt};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_database_connection() {
        let conn = Connection::establish("http://localhost:8529").await;
        assert!(conn.is_ok(), "Failed to connect to database");
        
        let conn = conn.unwrap();
        let db = conn.database("_system").await;
        assert!(db.is_ok(), "Failed to access system database");
    }

    #[tokio::test]
    async fn test_database_creation_and_deletion() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_db_integration";
        
        // Create test database
        let db = conn.create_database(test_db_name).await;
        assert!(db.is_ok(), "Failed to create test database");
        
        // Verify database exists
        let db_list = conn.list_databases().await;
        assert!(db_list.is_ok());
        let db_list = db_list.unwrap();
        assert!(db_list.contains(&test_db_name.to_string()));
        
        // Delete test database
        let result = conn.drop_database(test_db_name).await;
        assert!(result.is_ok(), "Failed to delete test database");
    }

    #[tokio::test]
    async fn test_collection_operations() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_collection_db";
        
        // Create test database
        let db = conn.create_database(test_db_name).await.unwrap();
        
        // Create test collection
        let collection_name = "test_collection";
        let collection = db.create_collection(collection_name).await;
        assert!(collection.is_ok(), "Failed to create collection");
        
        // Verify collection exists
        let collections = db.list_collections().await;
        assert!(collections.is_ok());
        let collections = collections.unwrap();
        assert!(collections.iter().any(|c| c.name == collection_name));
        
        // Delete collection
        let result = db.drop_collection(collection_name).await;
        assert!(result.is_ok(), "Failed to delete collection");
        
        // Cleanup - delete test database
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_document_operations() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_document_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await.unwrap();
        let collection = db.create_collection("test_docs").await.unwrap();
        
        // Test document insertion
        let test_doc = serde_json::json!({
            "name": "Test Document",
            "value": 42,
            "active": true
        });
        
        let insert_result = collection.create_document(test_doc.clone()).await;
        assert!(insert_result.is_ok(), "Failed to insert document");
        
        let inserted_doc = insert_result.unwrap();
        assert!(inserted_doc._id.is_some());
        
        // Test document retrieval
        let doc_id = inserted_doc._id.unwrap();
        let retrieved_doc = collection.document(&doc_id).await;
        assert!(retrieved_doc.is_ok(), "Failed to retrieve document");
        
        let retrieved_doc = retrieved_doc.unwrap();
        assert_eq!(retrieved_doc["name"], "Test Document");
        assert_eq!(retrieved_doc["value"], 42);
        assert_eq!(retrieved_doc["active"], true);
        
        // Test document update
        let update_doc = serde_json::json!({
            "name": "Updated Document",
            "value": 100,
            "active": false
        });
        
        let update_result = collection.update_document(&doc_id, update_doc).await;
        assert!(update_result.is_ok(), "Failed to update document");
        
        // Verify update
        let updated_doc = collection.document(&doc_id).await.unwrap();
        assert_eq!(updated_doc["name"], "Updated Document");
        assert_eq!(updated_doc["value"], 100);
        assert_eq!(updated_doc["active"], false);
        
        // Test document deletion
        let delete_result = collection.remove_document(&doc_id).await;
        assert!(delete_result.is_ok(), "Failed to delete document");
        
        // Verify deletion
        let deleted_doc = collection.document(&doc_id).await;
        assert!(deleted_doc.is_err(), "Document should not exist after deletion");
        
        // Cleanup
        let _ = db.drop_collection("test_docs").await;
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_query_operations() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_query_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await.unwrap();
        let collection = db.create_collection("test_query").await.unwrap();
        
        // Insert test documents
        let test_docs = vec![
            serde_json::json!({"name": "Doc 1", "value": 10, "category": "A"}),
            serde_json::json!({"name": "Doc 2", "value": 20, "category": "B"}),
            serde_json::json!({"name": "Doc 3", "value": 30, "category": "A"}),
            serde_json::json!({"name": "Doc 4", "value": 40, "category": "C"}),
        ];
        
        for doc in test_docs {
            let _ = collection.create_document(doc).await;
        }
        
        // Test simple query
        let query = "FOR doc IN test_query RETURN doc";
        let result = db.aql_query(query).await;
        assert!(result.is_ok(), "Failed to execute AQL query");
        
        let docs: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(docs.len(), 4);
        
        // Test filtered query
        let filtered_query = "FOR doc IN test_query FILTER doc.category == 'A' RETURN doc";
        let result = db.aql_query(filtered_query).await;
        assert!(result.is_ok(), "Failed to execute filtered AQL query");
        
        let filtered_docs: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(filtered_docs.len(), 2);
        
        // Test sorted query
        let sorted_query = "FOR doc IN test_query SORT doc.value ASC RETURN doc";
        let result = db.aql_query(sorted_query).await;
        assert!(result.is_ok(), "Failed to execute sorted AQL query");
        
        let sorted_docs: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(sorted_docs.len(), 4);
        assert_eq!(sorted_docs[0]["value"], 10);
        assert_eq!(sorted_docs[3]["value"], 40);
        
        // Test aggregated query
        let aggregate_query = "FOR doc IN test_query COLLECT category = doc.category WITH COUNT INTO count RETURN {category, count}";
        let result = db.aql_query(aggregate_query).await;
        assert!(result.is_ok(), "Failed to execute aggregate AQL query");
        
        let aggregated: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(aggregated.len(), 3); // 3 categories
        
        // Cleanup
        let _ = db.drop_collection("test_query").await;
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_transaction_operations() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_transaction_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await.unwrap();
        let collection = db.create_collection("test_transaction").await.unwrap();
        
        // Test transaction with multiple operations
        let transaction_query = r#"
            LET doc1 = INSERT {name: "Transaction Doc 1", value: 100} INTO test_transaction
            LET doc2 = INSERT {name: "Transaction Doc 2", value: 200} INTO test_transaction
            LET doc3 = INSERT {name: "Transaction Doc 3", value: 300} INTO test_transaction
            RETURN {doc1, doc2, doc3}
        "#;
        
        let result = db.aql_query(transaction_query).await;
        assert!(result.is_ok(), "Failed to execute transaction");
        
        // Verify all documents were inserted
        let count_query = "FOR doc IN test_transaction COLLECT WITH COUNT INTO count RETURN count";
        let result = db.aql_query(count_query).await;
        assert!(result.is_ok());
        
        let count: Vec<u64> = result.unwrap();
        assert_eq!(count[0], 3);
        
        // Cleanup
        let _ = db.drop_collection("test_transaction").await;
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_index_operations() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_index_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await.unwrap();
        let collection = db.create_collection("test_index").await.unwrap();
        
        // Create index
        let index_result = collection.create_index(&["name"]).await;
        assert!(index_result.is_ok(), "Failed to create index");
        
        // Verify index exists
        let indexes = collection.indexes().await;
        assert!(indexes.is_ok());
        let indexes = indexes.unwrap();
        assert!(indexes.iter().any(|idx| idx.fields.contains(&"name".to_string())));
        
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
            let _ = collection.create_document(doc).await;
        }
        
        // Test indexed query performance
        let start_time = std::time::Instant::now();
        let query = "FOR doc IN test_index FILTER doc.name == 'Doc 500' RETURN doc";
        let result = db.aql_query(query).await;
        let query_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Failed to execute indexed query");
        assert!(query_time.as_millis() < 100, "Indexed query should be fast");
        
        let docs: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["name"], "Doc 500");
        
        // Cleanup
        let _ = db.drop_collection("test_index").await;
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_connection_pooling() {
        let mut handles = Vec::new();
        
        // Test multiple concurrent connections
        for i in 0..10 {
            let handle = tokio::spawn(async move {
                let conn = Connection::establish("http://localhost:8529").await;
                assert!(conn.is_ok(), "Failed to establish connection {}", i);
                
                let conn = conn.unwrap();
                let db = conn.database("_system").await;
                assert!(db.is_ok(), "Failed to access system database from connection {}", i);
                
                // Simulate some work
                sleep(Duration::from_millis(100)).await;
                
                Ok::<(), Box<dyn std::error::Error>>(())
            });
            handles.push(handle);
        }
        
        // Wait for all connections to complete
        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok(), "Connection task failed");
            let result = result.unwrap();
            assert!(result.is_ok(), "Connection operation failed");
        }
    }

    #[tokio::test]
    async fn test_database_performance() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        let test_db_name = "test_performance_db";
        
        // Create test database and collection
        let db = conn.create_database(test_db_name).await.unwrap();
        let collection = db.create_collection("test_performance").await.unwrap();
        
        // Test bulk insert performance
        let start_time = std::time::Instant::now();
        
        let test_docs: Vec<serde_json::Value> = (0..10000)
            .map(|i| serde_json::json!({
                "id": i,
                "name": format!("Performance Doc {}", i),
                "value": i * 2,
                "timestamp": chrono::Utc::now().timestamp()
            }))
            .collect();
        
        // Insert documents in batches
        let batch_size = 1000;
        for batch in test_docs.chunks(batch_size) {
            let batch_docs: Vec<serde_json::Value> = batch.to_vec();
            let query = format!(
                "FOR doc IN {} INSERT doc INTO test_performance",
                serde_json::to_string(&batch_docs).unwrap()
            );
            let _ = db.aql_query(&query).await;
        }
        
        let insert_time = start_time.elapsed();
        assert!(insert_time.as_secs() < 30, "Bulk insert should complete within 30 seconds");
        
        // Test query performance
        let start_time = std::time::Instant::now();
        let query = "FOR doc IN test_performance FILTER doc.value > 5000 LIMIT 100 RETURN doc";
        let result = db.aql_query(query).await;
        let query_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Failed to execute performance query");
        assert!(query_time.as_millis() < 1000, "Query should complete within 1 second");
        
        let docs: Vec<serde_json::Value> = result.unwrap();
        assert_eq!(docs.len(), 100);
        
        // Cleanup
        let _ = db.drop_collection("test_performance").await;
        let _ = conn.drop_database(test_db_name).await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        
        // Test invalid database access
        let invalid_db = conn.database("nonexistent_database").await;
        assert!(invalid_db.is_err(), "Should fail to access non-existent database");
        
        // Test invalid AQL query
        let db = conn.database("_system").await.unwrap();
        let invalid_query = "INVALID AQL SYNTAX";
        let result = db.aql_query(invalid_query).await;
        assert!(result.is_err(), "Should fail to execute invalid AQL query");
        
        // Test invalid collection access
        let invalid_collection = db.collection("nonexistent_collection").await;
        assert!(invalid_collection.is_err(), "Should fail to access non-existent collection");
    }

    #[tokio::test]
    async fn test_database_cleanup() {
        let conn = Connection::establish("http://localhost:8529").await.unwrap();
        
        // List all databases
        let db_list = conn.list_databases().await.unwrap();
        
        // Clean up test databases
        for db_name in db_list {
            if db_name.starts_with("test_") {
                let _ = conn.drop_database(&db_name).await;
            }
        }
        
        // Verify cleanup
        let db_list_after = conn.list_databases().await.unwrap();
        let test_dbs: Vec<&String> = db_list_after.iter().filter(|name| name.starts_with("test_")).collect();
        assert_eq!(test_dbs.len(), 0, "All test databases should be cleaned up");
    }
}
