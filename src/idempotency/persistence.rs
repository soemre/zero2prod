use super::IdempotencyKey;
use actix_web::{body, http::StatusCode, HttpResponse};
use sqlx::{PgExecutor, PgPool, PgTransaction};
use uuid::Uuid;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub async fn get_saved_response(
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
    executor: impl PgExecutor<'_>,
) -> anyhow::Result<Option<HttpResponse>> {
    let saved_resp = sqlx::query!(
        r#"
        SELECT
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE user_id = $1
        AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(executor)
    .await?;

    match saved_resp {
        None => Ok(None),
        Some(r) => {
            let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
            let mut resp = HttpResponse::build(status_code);

            for HeaderPairRecord { name, value } in r.response_headers {
                resp.append_header((name, value));
            }

            let resp = resp.body(r.response_body);
            Ok(Some(resp))
        }
    }
}

pub async fn save_response(
    resp: HttpResponse,
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
    mut txn: PgTransaction<'static>,
) -> anyhow::Result<HttpResponse> {
    let (resp_head, body) = resp.into_parts();
    let status_code = resp_head.status().as_u16() as i16;
    let headers = resp_head
        .headers()
        .iter()
        .map(|(name, value)| HeaderPairRecord {
            name: name.as_str().into(),
            value: value.as_bytes().into(),
        })
        .collect::<Vec<_>>();
    let body = body::to_bytes(body)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET 
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE user_id = $1
            AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(txn.as_mut())
    .await?;
    txn.commit().await?;

    Ok(resp_head.set_body(body).map_into_boxed_body())
}

pub enum NextAction {
    StartProcessing(PgTransaction<'static>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
    pool: &PgPool,
) -> anyhow::Result<NextAction> {
    let mut txn = pool.begin().await?;
    let rows_affected = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(txn.as_mut())
    .await?
    .rows_affected();

    if rows_affected == 0 {
        let saved_resp = get_saved_response(user_id, idempotency_key, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_resp))
    } else {
        Ok(NextAction::StartProcessing(txn))
    }
}
