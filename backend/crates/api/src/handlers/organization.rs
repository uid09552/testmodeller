//! System, projects, components, features, tree and search.

use axum::extract::{RawQuery, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tm_storage::SearchType;
use uuid::Uuid;

use crate::dto::*;
use crate::error::{ApiError, ApiResult};
use crate::extract::{ApiJson, ApiPath, ApiQuery, IfMatch, Paging};
use crate::AppState;

/// `GET /health`
pub async fn health(State(state): State<AppState>) -> ApiResult<Json<Health>> {
    state.store.ping().await?;
    Ok(Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

/// `GET /projects`
pub async fn list_projects(
    State(state): State<AppState>,
    ApiQuery(paging): ApiQuery<Paging>,
) -> ApiResult<Json<PageDto<ProjectDto>>> {
    let page = state.store.list_projects(paging.to_request()?).await?;
    Ok(Json(PageDto::from_page(page)))
}

/// `POST /projects`
pub async fn create_project(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<NamedInput>,
) -> ApiResult<(StatusCode, Json<ProjectDto>)> {
    let p = state.store.create_project(input.into_fields()?).await?;
    Ok((StatusCode::CREATED, Json((&p).into())))
}

/// `GET /projects/{projectId}`
pub async fn get_project(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ProjectDto>> {
    Ok(Json((&state.store.get_project(id).await?).into()))
}

/// `PATCH /projects/{projectId}`
pub async fn update_project(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<NamedInput>,
) -> ApiResult<Json<ProjectDto>> {
    let p = state
        .store
        .update_project(id, version, input.into_fields()?)
        .await?;
    Ok(Json((&p).into()))
}

/// `DELETE /projects/{projectId}`
pub async fn delete_project(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_project(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /projects/{projectId}/tree`
pub async fn project_tree(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ProjectTreeDto>> {
    let tree = state.store.project_tree(id).await?;
    Ok(Json(tree.as_slice().into()))
}

/// `GET /projects/{projectId}/search?q=&types=`
///
/// `types` may be repeated (`types=model&types=feature`) or comma-separated.
pub async fn search(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    RawQuery(query): RawQuery,
) -> ApiResult<Json<Vec<SearchHitDto>>> {
    let mut q = None;
    let mut types = Vec::new();
    let pairs: Vec<(String, String)> =
        serde_urlencoded::from_str(query.as_deref().unwrap_or(""))
            .map_err(|e| ApiError::bad_request(format!("invalid query: {e}")))?;
    for (key, value) in pairs {
        match key.as_str() {
            "q" => q = Some(value),
            "types" | "types[]" => {
                for t in value.split(',').filter(|t| !t.is_empty()) {
                    types.push(match t {
                        "component" => SearchType::Component,
                        "feature" => SearchType::Feature,
                        "model" => SearchType::Model,
                        "testCase" => SearchType::TestCase,
                        other => {
                            return Err(ApiError::bad_request(format!("unknown type '{other}'")))
                        }
                    });
                }
            }
            _ => {}
        }
    }
    let q = q
        .filter(|q| !q.is_empty())
        .ok_or_else(|| ApiError::bad_request("query parameter q is required"))?;
    let hits = state.store.search(id, &q, &types).await?;
    Ok(Json(hits.iter().map(Into::into).collect()))
}

/// `GET /projects/{projectId}/components`
pub async fn list_components(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(paging): ApiQuery<Paging>,
) -> ApiResult<Json<PageDto<ComponentDto>>> {
    let page = state
        .store
        .list_components(id, paging.to_request()?)
        .await?;
    Ok(Json(PageDto::from_page(page)))
}

/// `POST /projects/{projectId}/components`
pub async fn create_component(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<NamedInput>,
) -> ApiResult<(StatusCode, Json<ComponentDto>)> {
    let c = state
        .store
        .create_component(id, input.into_fields()?)
        .await?;
    Ok((StatusCode::CREATED, Json((&c).into())))
}

/// `GET /components/{componentId}`
pub async fn get_component(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ComponentDto>> {
    Ok(Json((&state.store.get_component(id).await?).into()))
}

/// `PATCH /components/{componentId}`
pub async fn update_component(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<NamedInput>,
) -> ApiResult<Json<ComponentDto>> {
    let c = state
        .store
        .update_component(id, version, input.into_fields()?)
        .await?;
    Ok(Json((&c).into()))
}

/// `cascade` query parameter.
#[derive(Debug, Deserialize)]
pub struct CascadeQuery {
    #[serde(default)]
    cascade: bool,
}

/// `DELETE /components/{componentId}`
pub async fn delete_component(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<CascadeQuery>,
) -> ApiResult<StatusCode> {
    state.store.delete_component(id, q.cascade).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Query of `GET /components/{componentId}/features`.
#[derive(Debug, Deserialize)]
pub struct FeatureListQuery {
    limit: Option<i64>,
    cursor: Option<String>,
    tag: Option<String>,
}

/// `GET /components/{componentId}/features`
pub async fn list_features(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<FeatureListQuery>,
) -> ApiResult<Json<PageDto<FeatureDto>>> {
    let paging = Paging::new(q.limit, q.cursor).to_request()?;
    let page = state
        .store
        .list_features(id, q.tag.as_deref(), paging)
        .await?;
    Ok(Json(PageDto::from_page(page)))
}

/// `POST /components/{componentId}/features`
pub async fn create_feature(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<FeatureInput>,
) -> ApiResult<(StatusCode, Json<FeatureDto>)> {
    let f = state.store.create_feature(id, input.into_fields()?).await?;
    Ok((StatusCode::CREATED, Json((&f).into())))
}

/// `GET /features/{featureId}`
pub async fn get_feature(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<FeatureDto>> {
    Ok(Json((&state.store.get_feature(id).await?).into()))
}

/// `PATCH /features/{featureId}`
pub async fn update_feature(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<FeatureInput>,
) -> ApiResult<Json<FeatureDto>> {
    let f = state
        .store
        .update_feature(id, version, input.into_fields()?)
        .await?;
    Ok(Json((&f).into()))
}

/// `DELETE /features/{featureId}`
pub async fn delete_feature(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<CascadeQuery>,
) -> ApiResult<StatusCode> {
    state.store.delete_feature(id, q.cascade).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /features/{featureId}/move`
pub async fn move_feature(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<MoveFeature>,
) -> ApiResult<Json<FeatureDto>> {
    let f = state.store.move_feature(id, body.component_id).await?;
    Ok(Json((&f).into()))
}
