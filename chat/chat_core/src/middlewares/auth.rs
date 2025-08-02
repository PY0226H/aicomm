use axum::{
    extract::{FromRequestParts, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::TokenVerify;

#[derive(Debug, Serialize, Deserialize)]
struct Params {
    token: String,
}

pub async fn verify_token<T>(State(state): State<T>, req: Request, next: Next) -> Response
where
    T: TokenVerify + Clone + Send + Sync + 'static,
{
    let (mut parts, body) = req.into_parts();
    let token =
        match TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await {
            Ok(TypedHeader(Authorization(bearer))) => bearer.token().to_string(),
            Err(e) => {
                if e.is_missing() {
                    match Query::<Params>::from_request_parts(&mut parts, &state).await {
                        Ok(params) => params.token.clone(),
                        Err(e) => {
                            let msg = format!("parse query params failed: {}", e);
                            warn!(msg);
                            return (StatusCode::UNAUTHORIZED, msg).into_response();
                        }
                    }
                } else {
                    let msg = format!("parse Authorization header failed: {}", e);
                    warn!(msg);
                    return (StatusCode::UNAUTHORIZED, msg).into_response();
                }
            }
        };

    let req = match state.verify(&token) {
        Ok(user) => {
            let mut req = Request::from_parts(parts, body);
            req.extensions_mut().insert(user);
            req
        }
        Err(e) => {
            let msg = format!("verify token failed: {:?}", e);
            warn!(msg);
            return (StatusCode::FORBIDDEN, msg).into_response();
        }
    };

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{DecodingKey, EncodingKey, User};

    use super::*;
    use anyhow::Result;
    use axum::{Router, body::Body, middleware::from_fn_with_state, routing::get};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct AppState(Arc<AppStateInner>);

    struct AppStateInner {
        ek: EncodingKey,
        dk: DecodingKey,
    }

    impl TokenVerify for AppState {
        type Error = ();

        fn verify(&self, token: &str) -> Result<User, Self::Error> {
            self.0.dk.verify(token).map_err(|_| ())
        }
    }

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    #[tokio::test]
    async fn verify_token_middleware_should_work() -> Result<()> {
        let state = AppState(Arc::new(AppStateInner {
            ek: EncodingKey::load(include_str!("../../fixtures/encoding.pem"))?,
            dk: DecodingKey::load(include_str!("../../fixtures/decoding.pem"))?,
        }));
        let user = User::new(1, "Tyr Chen", "tchen@acme.org");

        let token = state.0.ek.sign(user)?;

        let app = Router::new()
            .route("/", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);

        // good token
        let req = Request::builder()
            .uri("/")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        //good token in query params
        let req = Request::builder()
            .uri(format!("/?token={}", token))
            .body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // no token
        let req = Request::builder().uri("/").body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        //bad token
        let req = Request::builder()
            .uri("/")
            .header("authorization", "Bearer badtoken")
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        //bad token in query params
        let req = Request::builder()
            .uri("/?token=bad-token")
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }
}
