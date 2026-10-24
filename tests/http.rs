mod common;

use common::TestService;
use reqwest::{Method, StatusCode};

#[tokio::test]
async fn petstore_supports_crud_lifecycle() {
    let app = TestService::start("local", false).await;
    let response = app.request(Method::POST, "/pet")
.header("content-type", "application/json")
.body(r#"{"name":"Milo","photoUrls":["https://example.test/milo"],"tags":["cat"],"status":"available"}"#)
.send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app.request(Method::GET, "/pet/1").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.bytes().await.unwrap();
    assert!(body.windows(b"Milo".len()).any(|window| window == b"Milo"));

    let response = app
        .request(Method::PUT, "/pet/1")
        .header("content-type", "application/json")
        .body(r#"{"name":"Milo updated","tags":["cat","friendly"],"status":"sold"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app.request(Method::DELETE, "/pet/1").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app.request(Method::GET, "/pet/1").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn petstore_validates_json_and_pet_names() {
    let app = TestService::start("local", false).await;
    let response = app
        .request(Method::POST, "/pet")
        .header("content-type", "application/json")
        .body(r#"{"name":""}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .request(Method::POST, "/pet")
        .header("content-type", "application/json")
        .body(r#"{"name":"Milo""#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn petstore_filters_by_status_and_tags() {
    let app = TestService::start("local", false).await;
    for payload in [
        r#"{"name":"Milo","tags":["cat"],"status":"available"}"#,
        r#"{"name":"Rex","tags":["dog"],"status":"sold"}"#,
    ] {
        let response = app
            .request(Method::POST, "/pet")
            .header("content-type", "application/json")
            .body(payload)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = app
        .request(Method::GET, "/pet/findByStatus?status=available")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.bytes().await.unwrap();
    assert!(body.windows(b"Milo".len()).any(|window| window == b"Milo"));
    assert!(!body.windows(b"Rex".len()).any(|window| window == b"Rex"));

    let response = app
        .request(Method::GET, "/pet/findByTags?tags=dog")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.bytes().await.unwrap();
    assert!(body.windows(b"Rex".len()).any(|window| window == b"Rex"));
}
