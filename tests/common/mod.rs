use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use reqwest::{Client, Method, RequestBuilder};

pub struct TestService {
    child: Child,
    client: Client,
    pub base_url: String,
    pub bootui_url: String,
}

impl TestService {
    pub async fn start(profile: &str, bootui: bool) -> Self {
        // Reserve distinct ports together, then release them for Scafra to bind.
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let dashboard = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_port = server.local_addr().unwrap().port();
        let bootui_port = dashboard.local_addr().unwrap().port();
        let mut command = Command::new(env!("CARGO_BIN_EXE_scafra-test-service"));
        // Keep local development overrides out of the test process.
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("SCAFRA_") {
                command.env_remove(key);
            }
        }
        command
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("SCAFRA_CONFIG_DIR", "src/resources")
            .env("SCAFRA_PROFILE", profile)
            .env("SCAFRA_DEV_RELOAD", "false")
            .env("SCAFRA_SERVER_HOST", "127.0.0.1")
            .env("SCAFRA_SERVER_PORT", server_port.to_string())
            .env("SCAFRA_BOOTUI_ENABLED", bootui.to_string())
            .env("SCAFRA_BOOTUI_PORT", bootui_port.to_string())
            .env("SCAFRA_STARTUP_BANNER", "false")
            .env("SCAFRA_STARTUP_SHOW_CONFIG", "false")
            .stdout(Stdio::null());
        drop((server, dashboard));
        let mut service = Self {
            child: command.spawn().expect("service should start"),
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
            base_url: format!("http://127.0.0.1:{server_port}"),
            bootui_url: format!("http://127.0.0.1:{bootui_port}/bootui"),
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(
                service.child.try_wait().unwrap().is_none(),
                "service exited before becoming ready"
            );
            let ready = service
                .request(Method::GET, "/ready")
                .basic_auth("demo", Some("scafra-local"))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success());
            let dashboard_ready = !bootui
                || service
                    .dashboard("/api/config")
                    .send()
                    .await
                    .is_ok_and(|response| response.status().is_success());
            if ready && dashboard_ready {
                return service;
            }
            assert!(Instant::now() < deadline, "service startup timed out");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.base_url))
    }

    pub fn dashboard(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}{path}", self.bootui_url))
    }
}

impl Drop for TestService {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
