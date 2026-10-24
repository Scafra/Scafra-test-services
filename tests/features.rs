use std::path::PathBuf;

use scafra::web::{
    axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        Router,
    },
    tower::util::ServiceExt,
};
use scafra_test_service as _;

fn dev_config() -> scafra::config::ScafraConfig {
    scafra::ConfigLoader::new()
        .root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/resources"))
        .env_prefix("SCAFRA_FEATURE_TEST")
        .profile("dev")
        .load()
        .expect("dev profile should load")
}

#[tokio::test]
async fn dev_profile_features_are_wired_through_scafra() {
    let config = dev_config();
    assert_eq!(config.server.port, 8081);
    assert!(config.security.enabled);
    assert_eq!(config.security.basic.username.as_deref(), Some("demo"));
    assert!(config.bootui.enabled);
    assert_eq!(
        config.scheduler.tasks["pet-maintenance"].interval_ms,
        Some(5_000)
    );
    assert!(config.actuator.is_enabled("metrics"));
    assert_eq!(config.actuator.health.checks, ["pet-store".to_owned()]);

    let app =
        scafra::web::build_router_with_actuator_and_security(&config.actuator, &config.security)
            .expect("actuator router should build");

    let response = app
        .clone()
        .oneshot(get_request("/metrics", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .clone()
        .oneshot(get_request(
            "/metrics",
            Some("Basic ZGVtbzpzY2FmcmEtbG9jYWw="),
        ))
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(body
        .windows(b"scafra_up 1".len())
        .any(|window| window == b"scafra_up 1"));

    let response = app
        .oneshot(get_request(
            "/ready",
            Some("Basic ZGVtbzpzY2FmcmEtbG9jYWw="),
        ))
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.windows(b"DOWN".len()).any(|window| window == b"DOWN"));

    let pet_app = scafra_test_service::build_petstore_router(&config)
        .expect("secured Pet API router should build");
    let response = pet_app
        .clone()
        .oneshot(get_request(
            "/ready",
            Some("Basic ZGVtbzpzY2FmcmEtbG9jYWw="),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = pet_app
        .clone()
        .oneshot(get_request("/pet", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = pet_app
        .oneshot(get_request("/pet", Some("Basic ZGVtbzpzY2FmcmEtbG9jYWw=")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // BootUI requires the peer address to verify that the request is local.
    let dashboard = scafra_bootui::try_layer_with_config_and_profile(Router::new(), &config, "dev")
        .expect("BootUI should accept the dev configuration")
        .layer(scafra::web::axum::Extension(
            scafra::web::axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                12345,
            ))),
        ));
    let response = dashboard
        .clone()
        .oneshot(get_request("/bootui", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(body
        .windows(b"Scafra BootUI".len())
        .any(|window| window == b"Scafra BootUI"));

    let response = dashboard
        .oneshot(get_request("/bootui/api/config", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("\"server\""), "{body}");
    assert!(body.contains("\"port\":8081"), "{body}");
    assert!(body.contains("\"profile\":\"dev\""), "{body}");
    assert!(!body.contains("scafra-local"), "{body}");
}

fn get_request(path: &str, authorization: Option<&str>) -> Request<Body> {
    let mut request = Request::get(path);
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    request.body(Body::empty()).unwrap()
}
