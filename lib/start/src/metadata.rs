const APP_VAR: &str = "APP_NAME";
const REGION_VAR: &str = "REGION";
const HOST_VAR: &str = "HOST";

pub struct AppMeta {
    pub app: String,
    pub region: String,
    pub host: String,
}

impl AppMeta {
    pub fn new(app: impl AsRef<str>, region: impl AsRef<str>, host: impl AsRef<str>) -> Self {
        Self {
            app: app.as_ref().to_owned(),
            region: region.as_ref().to_owned(),
            host: host.as_ref().to_owned(),
        }
    }

    pub fn from_env() -> Self {
        let app = std::env::var(APP_VAR).unwrap_or_default();
        let region = std::env::var(REGION_VAR).unwrap_or_default();
        let host = std::env::var(HOST_VAR).unwrap_or_default();
        Self::new(app, region, host)
    }

    pub fn with_app_name(mut self, app: impl AsRef<str>) -> Self {
        self.app = app.as_ref().to_owned();
        self
    }
}
