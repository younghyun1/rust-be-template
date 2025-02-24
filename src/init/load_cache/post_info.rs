use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tracing::error;

use crate::{domain::blog::PostInfo, init::state::ServerState, schema::posts};

pub async fn load_post_info(state: &ServerState) -> anyhow::Result<Vec<PostInfo>> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = %e, "Could not get conn out of pool while synchronizing state.");
            return Err(e);
        }
    };

    let post_infos: Vec<PostInfo> = match posts::table
        .select(PostInfo::as_select())
        .order(posts::post_created_at.desc())
        .load::<PostInfo>(&mut conn)
        .await
    {
        Ok(post_infos) => post_infos,
        Err(e) => return Err(e.into()),
    };

    drop(conn);

    Ok(post_infos)
}
