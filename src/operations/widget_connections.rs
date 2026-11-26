//! Non-secret picker data shared by Android widget entry points.
use crate::configuration::AppConfig;
use serde::Serialize;
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct WidgetConnection {
    pub id: String,
    pub name: String,
    pub verified: bool,
    pub requires_opt_in: bool,
}
pub fn available(config: &AppConfig) -> Vec<WidgetConnection> {
    config
        .connections
        .iter()
        .filter(|c| !c.show_preview || c.mode != crate::configuration::SyncMode::Archive)
        .map(|c| WidgetConnection {
            id: c.id.as_str().into(),
            name: c.name.clone(),
            verified: c.verified,
            requires_opt_in: c.show_preview,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_flags_preview_connections_and_omits_preview_archives() {
        let mut config = crate::configuration::lists::tests::config();
        config.connections[0].show_preview = true;
        assert!(available(&config)[0].requires_opt_in);
        config.connections[0].mode = crate::configuration::SyncMode::Archive;
        assert!(available(&config).is_empty());
        config.connections[0].show_preview = false;
        assert!(available(&config)[0].verified);
        assert!(!available(&config)[0].requires_opt_in);
        config.connections[0].verified = false;
        let eligible = available(&config);
        assert!(!eligible[0].verified);
        assert_eq!(eligible[0].name, config.connections[0].name);
        assert_eq!(eligible[0].id, config.connections[0].id.as_str());
    }
}
