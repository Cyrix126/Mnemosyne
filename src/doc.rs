use aide::{axum::IntoApiResponse, openapi::OpenApi, transform::TransformOpenApi};
use axum::Extension;
use std::sync::Arc;

/// serve document as json
pub async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    axum::Json(api)
}

/// description OpenAPI document
pub fn description_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Mnemosyne Open API")
        .summary("Caching proxy server OpenAPI")
        .description(include_str!("../README.md"))
}
