mod common;

use std::path::PathBuf;

use common::TestService;
use reqwest::{Method, StatusCode};

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

    let app = TestService::start("dev", true).await;
    for path in ["/metrics", "/pet"] {
        let response = app.request(Method::GET, path).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = app
        .request(Method::GET, "/metrics")
        .basic_auth("demo", Some("scafra-local"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().contains("scafra_up 1"));

    // Readiness must observe the PetStore created by Scafra's application graph.
    for path in ["/ready", "/pet"] {
        let response = app
            .request(Method::GET, path)
            .basic_auth("demo", Some("scafra-local"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app.dashboard("").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().contains("Scafra BootUI"));

    let response = app.dashboard("/api/config").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let port = app.base_url.rsplit(':').next().unwrap();
    assert!(body.contains(&format!("\"port\":{port}")), "{body}");
    assert!(body.contains("\"profile\":\"dev\""), "{body}");
    assert!(!body.contains("scafra-local"), "{body}");
}
