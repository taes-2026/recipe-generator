use std::net::SocketAddr;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;

use recipe_generator::{build_router, build_state, Config};

const PAYLOAD: &str = r#"{
    "ingredients": [
        { "name": "Tomate", "quantity": 2, "unit": "unit" },
        { "name": "Cebola", "quantity": 1, "unit": "unit" }
    ],
    "pantry": ["Arroz", "Azeite", "Sal"],
    "profile": {
        "servings": 2,
        "diet": "omnivore",
        "intolerances": [],
        "restrictions": []
    }
}"#;

fn router_with(config: Config) -> Router {
    build_router(build_state(config).expect("state builds"))
}

fn testing_router() -> Router {
    router_with(Config::default())
}

fn post_request(body: &str) -> Request<Body> {
    let mut request = Request::post("/v1/sous-chef")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request builds");

    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

    request
}

async fn call(router: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, value)
}

#[tokio::test]
async fn healthz_reports_the_active_mode() {
    let request = Request::get("/healthz")
        .body(Body::empty())
        .expect("request builds");

    let (status, body) = call(testing_router(), request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["mode"], "testing");
}

#[tokio::test]
async fn testing_mode_generates_a_complete_recipe() {
    let (status, body) = call(testing_router(), post_request(PAYLOAD)).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["meta"]["source"], "testing");
    assert_eq!(body["recipe"]["servings"], 2);
    assert!(body["recipe"]["title"]
        .as_str()
        .is_some_and(|title| !title.is_empty()));

    let ingredients: Vec<String> = body["recipe"]["ingredients"]
        .as_array()
        .expect("ingredients array")
        .iter()
        .map(|ingredient| {
            ingredient["name"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect();

    assert!(ingredients.iter().any(|name| name == "tomate"));
    assert!(body["id"].as_str().is_some_and(|id| id.starts_with("ai-")));

    for step in body["recipe"]["steps"].as_array().expect("steps array") {
        let uses = step["uses"].as_array().expect("uses array");
        assert!(!uses.is_empty(), "every testing step lists ingredients");

        for used in uses {
            let used = used.as_str().unwrap_or_default().to_lowercase();
            assert!(
                ingredients.contains(&used),
                "step uses '{used}', which is not in the ingredient list"
            );
        }
    }
}

#[tokio::test]
async fn rejects_zero_servings() {
    let payload = PAYLOAD.replace("\"servings\": 2", "\"servings\": 0");
    let (status, body) = call(testing_router(), post_request(&payload)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_input");
    assert!(body["error"]["details"]
        .as_array()
        .is_some_and(|details| !details.is_empty()));
}

#[tokio::test]
async fn rejects_an_empty_leftover_list() {
    let payload = r#"{
        "ingredients": [],
        "pantry": ["Arroz"],
        "profile": { "servings": 2, "diet": "omnivore" }
    }"#;

    let (status, body) = call(testing_router(), post_request(payload)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn rejects_an_unknown_unit() {
    let payload = PAYLOAD.replace("\"unit\": \"unit\"", "\"unit\": \"colher\"");
    let (status, body) = call(testing_router(), post_request(&payload)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn rejects_malformed_json_with_the_same_envelope() {
    let (status, body) = call(testing_router(), post_request("{ not json")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn rate_limits_after_the_configured_quota() {
    let router = router_with(Config {
        rate_limit_per_minute: 1,
        ..Config::default()
    });

    let (first, _) = call(router.clone(), post_request(PAYLOAD)).await;
    let (second, body) = call(router, post_request(PAYLOAD)).await;

    assert_eq!(first, StatusCode::OK);
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"]["code"], "rate_limited");
}
