use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub async fn create_db_pool(db_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(db_url).await?;
    
    // Create the currencies table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS currencies (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn insert_currency(pool: &SqlitePool, code: &str, name: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO currencies (code, name)
        VALUES (?, ?)
        ON CONFLICT(code) DO UPDATE SET
            name = excluded.name,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(code)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_currency(pool: &SqlitePool, code: &str) -> Result<Option<(String, String)>> {
    let record = sqlx::query!(
        r#"
        SELECT code, name
        FROM currencies
        WHERE code = ?
        "#,
        code
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| (r.code.unwrap_or_default(), r.name)))
}

pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    let records = sqlx::query!(
        r#"
        SELECT code, name
        FROM currencies
        ORDER BY code
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(|r| (r.code.unwrap_or_default(), r.name)).collect())
}
