use super::models::{AuditLog, NewAuditLog};
use sqlx::PgPool;

pub async fn list_recent(pool: &PgPool, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as!(
        AuditLog,
        r#"
        SELECT id, occurred_at, actor_id, actor_username, method, path, status_code, source_ip
        FROM audit_logs
        ORDER BY occurred_at DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

pub async fn insert(pool: &PgPool, entry: NewAuditLog) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (actor_id, actor_username, method, path, status_code, source_ip)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        entry.actor_id,
        entry.actor_username,
        entry.method,
        entry.path,
        entry.status_code,
        entry.source_ip
    )
    .execute(pool)
    .await?;

    Ok(())
}
