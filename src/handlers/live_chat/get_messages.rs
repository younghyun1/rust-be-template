use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::live_chat::{cache::CachedChatMessage, message::LiveChatMessage},
    dto::{
        requests::live_chat::GetLiveChatMessagesRequest,
        responses::{
            live_chat::{GetLiveChatMessagesResponse, LiveChatMessageItem},
            response_data::http_resp,
        },
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::live_chat_messages,
    util::time::now::tokio_now,
};

const MAX_LIVE_CHAT_PAGE_SIZE: usize = 100;

#[utoipa::path(
    get,
    path = "/api/live-chat/messages",
    tag = "live_chat",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of messages"),
        ("before_message_id" = Option<Uuid>, Query, description = "Cursor message ID")
    ),
    responses(
        (status = 200, description = "Live chat messages", body = GetLiveChatMessagesResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_live_chat_messages(
    State(state): State<Arc<ServerState>>,
    Query(request): Query<GetLiveChatMessagesRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let limit = request.limit.clamp(1, MAX_LIVE_CHAT_PAGE_SIZE);

    let messages = match request.before_message_id {
        Some(before_message_id) => {
            get_messages_before_from_db(state.clone(), before_message_id, limit).await?
        }
        None => state.live_chat_cache.get_recent_chat_messages(limit).await,
    };

    let has_more = messages.len() == limit;
    let next_before_message_id = messages
        .as_slice()
        .first()
        .map(|message| message.live_chat_message_id);
    let items = messages
        .into_iter()
        .map(LiveChatMessageItem::from)
        .collect::<Vec<LiveChatMessageItem>>();

    Ok(http_resp(
        GetLiveChatMessagesResponse {
            items,
            next_before_message_id,
            has_more,
        },
        (),
        start,
    ))
}

async fn get_messages_before_from_db(
    state: Arc<ServerState>,
    before_message_id: Uuid,
    limit: usize,
) -> HandlerResponse<Vec<CachedChatMessage>> {
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let before_row: Option<LiveChatMessage> = live_chat_messages::table
        .filter(live_chat_messages::live_chat_message_id.eq(before_message_id))
        .select(LiveChatMessage::as_select())
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let before_row = match before_row {
        Some(row) => row,
        None => {
            return Err(code_err(
                CodeError::INVALID_REQUEST,
                "before_message_id does not exist",
            ));
        }
    };

    let db_limit = i64::try_from(limit).map_err(|e| code_err(CodeError::INVALID_REQUEST, e))?;

    let mut rows: Vec<LiveChatMessage> = live_chat_messages::table
        .filter(live_chat_messages::room_key.eq(before_row.room_key))
        .filter(live_chat_messages::message_deleted_at.is_null())
        .filter(
            live_chat_messages::message_created_at
                .lt(before_row.message_created_at)
                .or(live_chat_messages::message_created_at
                    .eq(before_row.message_created_at)
                    .and(live_chat_messages::live_chat_message_id.lt(before_message_id))),
        )
        .select(LiveChatMessage::as_select())
        .order((
            live_chat_messages::message_created_at.desc(),
            live_chat_messages::live_chat_message_id.desc(),
        ))
        .limit(db_limit)
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    drop(conn);
    rows.reverse();

    Ok(rows
        .into_iter()
        .map(CachedChatMessage::from)
        .collect::<Vec<CachedChatMessage>>())
}
