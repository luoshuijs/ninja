use std::str::FromStr;

use axum::body::Bytes;
use axum::{
    async_trait,
    http::{self},
};
use http::header;
use http::{HeaderMap, Method};
use serde_json::{json, Value};

use crate::arkose::{ArkoseContext, ArkoseToken, Type};
use crate::constant::{ARKOSE_TOKEN, EMPTY, MODEL, NULL, PUID};
use crate::gpt_model::GPTModel;
use crate::{arkose, info, warn, with_context};

use super::ext::{RequestExt, ResponseExt, SendRequestExt};
use super::header_convert;
use super::toapi;
use crate::serve::error::{ProxyError, ResponseError};
use crate::serve::puid::{get_or_init, reduce_key};
use crate::URL_CHATGPT_API;

#[async_trait]
impl SendRequestExt for reqwest::Client {
    async fn send_request(
        &self,
        origin: &'static str,
        mut req: RequestExt,
    ) -> Result<ResponseExt, ResponseError> {
        // If to_api is true, then send request to api
        if toapi::support(&req) {
            return toapi::send_request(req).await;
        }

        // Build rqeuest path and query
        let path_and_query = req
            .uri
            .path_and_query()
            .map(|v| v.as_str())
            .unwrap_or(req.uri.path());

        // Build url
        let url = format!("{origin}{path_and_query}");

        // Handle conversation request
        handle_conv_request(&mut req).await?;

        // Handle dashboard request
        handle_dashboard_request(&mut req).await?;

        // Build request
        let mut builder =
            self.request(req.method, url)
                .headers(header_convert(&req.headers, &req.jar, origin)?);
        if let Some(body) = req.body {
            builder = builder.body(body);
        }

        // Send request
        Ok(ResponseExt::builder().inner(builder.send().await?).build())
    }
}

/// Check if the request has puid
pub(super) fn has_puid(headers: &HeaderMap) -> Result<bool, ResponseError> {
    if let Some(hv) = headers.get(header::COOKIE) {
        let cookie_str = hv.to_str().map_err(ResponseError::BadRequest)?;
        Ok(cookie_str.contains(PUID))
    } else {
        Ok(false)
    }
}

/// Handle conversation request
async fn handle_conv_request(req: &mut RequestExt) -> Result<(), ResponseError> {
    // Only handle POST request
    if !(req.uri.path().eq("/backend-api/conversation") && req.method.eq(&Method::POST)) {
        return Ok(());
    }

    // Handle empty body
    let body = req
        .body
        .as_ref()
        .ok_or(ResponseError::BadRequest(ProxyError::BodyRequired))?;

    // Use serde_json to parse body
    let mut json = serde_json::from_slice::<Value>(&body).map_err(ResponseError::BadRequest)?;
    let body = json
        .as_object_mut()
        .ok_or(ResponseError::BadRequest(ProxyError::BodyMustBeJsonObject))?;

    // If model is not exist, then return error
    let model = body
        .get(MODEL)
        .and_then(|m| m.as_str())
        .ok_or(ResponseError::BadRequest(ProxyError::ModelRequired))?;

    // extract token from Authorization header
    let token = req
        .bearer_auth()
        .ok_or(ResponseError::Unauthorized(ProxyError::AccessTokenRequired))?
        .to_owned();

    // If puid is exist, then return
    if !has_puid(&req.headers)? {
        // Exstract the token from the Authorization header
        let cache_id = reduce_key(&token)?;

        // Get or init puid
        let puid = get_or_init(&token, model, cache_id).await?;

        if let Some(puid) = puid {
            req.headers.insert(
                header::COOKIE,
                header::HeaderValue::from_str(&format!("{PUID}={puid};"))
                    .map_err(ResponseError::BadRequest)?,
            );
        }
    }

    let chat_requirements_token = create_chat_requirements_token(&token).await?;
    if let Some(chat_requirements_token) = chat_requirements_token {
        req.headers.insert(
            header::HeaderName::from_static("openai-sentinel-chat-requirements-token"),
            header::HeaderValue::from_str(chat_requirements_token.as_str())
                .map_err(ResponseError::BadRequest)?,
        );
        info!("Chat requirements token: {}", chat_requirements_token.as_str())
    }
    else {
        warn!("Chat requirements token not found")
    }

    // Parse model
    let model = GPTModel::from_str(model).map_err(ResponseError::BadRequest)?;

    // If model is gpt3 or gpt4, then add arkose_token
    if (with_context!(arkose_gpt3_experiment) && model.is_gpt3()) || model.is_gpt4() {
        let condition = match body.get(ARKOSE_TOKEN) {
            Some(s) => {
                let s = s.as_str().unwrap_or(EMPTY);
                s.is_empty() || s.eq(NULL)
            }
            None => true,
        };

        if condition {
            let arkose_token = ArkoseToken::new_from_context(
                ArkoseContext::builder()
                    .client(with_context!(arkose_client))
                    .typed(model.into())
                    .identifier(Some(token))
                    .build(),
            )
            .await?;
            body.insert(ARKOSE_TOKEN.to_owned(), json!(arkose_token.value()));
            // Updaye Modify bytes
            req.body = Some(Bytes::from(
                serde_json::to_vec(&json).map_err(ResponseError::BadRequest)?,
            ));
        }
    }

    drop(json);

    Ok(())
}

/// Handle dashboard request
async fn handle_dashboard_request(req: &mut RequestExt) -> Result<(), ResponseError> {
    // Only handle POST request
    if !(req.uri.path().eq("/dashboard/user/api_keys") && req.method.eq(&Method::POST)) {
        return Ok(());
    }

    // Handle empty body
    let body = req
        .body
        .as_ref()
        .ok_or(ResponseError::BadRequest(ProxyError::BodyRequired))?;

    // Use serde_json to parse body
    let mut json = serde_json::from_slice::<Value>(&body).map_err(ResponseError::BadRequest)?;
    let body = json
        .as_object_mut()
        .ok_or(ResponseError::BadRequest(ProxyError::BodyMustBeJsonObject))?;

    // If arkose_token is not exist, then add it
    if body.get(ARKOSE_TOKEN).is_none() {
        let arkose_token = arkose::ArkoseToken::new_from_context(
            arkose::ArkoseContext::builder()
                .client(with_context!(arkose_client))
                .typed(Type::Platform)
                .identifier(None)
                .build(),
        )
        .await?;
        body.insert(ARKOSE_TOKEN.to_owned(), json!(arkose_token.value()));
        // Updaye Modify bytes
        req.body = Some(Bytes::from(
            serde_json::to_vec(&json).map_err(ResponseError::BadRequest)?,
        ));
    }

    drop(json);

    Ok(())
}

async fn create_chat_requirements_token(token: &str) -> Result<Option<String>, ResponseError> {
    let token = token.trim_start_matches("Bearer ");
    let resp = with_context!(api_client)
        .post(format!(
            "{URL_CHATGPT_API}/backend-api/sentinel/chat-requirements"
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(ResponseError::InternalServerError)?
        .error_for_status()
        .map_err(ResponseError::BadRequest)?;
    let body = resp.bytes().await?;
    let json = serde_json::from_slice::<Value>(&body).map_err(ResponseError::BadRequest)?;
    if let Some(token_value) = json.get("token") {
        if let Some(token_str) = token_value.as_str() {
            return Ok(Some(token_str.to_owned()));
        }
    }
    Ok(None)
}
