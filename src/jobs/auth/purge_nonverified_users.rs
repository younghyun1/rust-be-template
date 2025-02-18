use std::sync::Arc;

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::{error, info};

use crate::{
    init::state::ServerState,
    schema::{email_verification_tokens, users},
};

pub async fn purge_nonverified_users(state: Arc<ServerState>) {
    let now = chrono::Utc::now();

    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = %e, "Failed to get connection from pool to purge non-verified users");
            return;
        }
    };

    match diesel::delete(
        users::table.filter(
            users::user_id.eq_any(
                email_verification_tokens::table
                    .select(email_verification_tokens::user_id)
                    .filter(email_verification_tokens::email_verification_token_expires_at.lt(now)),
            ),
        ),
    )
    .execute(&mut conn)
    .await
    {
        Ok(number_of_users_deleted) => {
            info!(number_of_users_deleted = %number_of_users_deleted, "Non-verified users with expired verification tokens were deleted");
        }
        Err(e) => {
            error!(error = %e, "Failed to purge non-verified users");
        }
    };

    drop(conn);
}
