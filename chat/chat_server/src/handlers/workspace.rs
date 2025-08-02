use axum::{Extension, Json, extract::State, response::IntoResponse};

use crate::{AppError, AppState};
use chat_core::{ChatUser, User};

#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of ws users", body = Vec<ChatUser>),
    ),
    security(
        ("token" = [])
    )
)]
/// List all users in the workspace.
pub(crate) async fn list_chat_users_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.fetch_chat_users(user.ws_id as _).await?;
    Ok(Json(users))
}
