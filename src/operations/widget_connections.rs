//! Non-secret picker data shared by Android widget entry points.
use crate::configuration::AppConfig;
use serde::Serialize;
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct WidgetConnection {
    pub id: String,
    pub name: String,
    pub verified: bool,
}
pub fn available(config: &AppConfig) -> Vec<WidgetConnection> {
    config
        .connections
        .iter()
        .filter(|c| !c.show_preview)
        .map(|c| WidgetConnection {
            id: c.id.as_str().into(),
            name: c.name.clone(),
            verified: c.verified,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_omits_preview_connections_and_preserves_verification_status() {
        let mut config = crate::configuration::lists::tests::config();
        assert!(available(&config).is_empty());
        config.connections[0].show_preview = false;
        assert!(available(&config)[0].verified);
        config.connections[0].verified = false;
        let eligible = available(&config);
        assert!(!eligible[0].verified);
        assert_eq!(eligible[0].name, config.connections[0].name);
        assert_eq!(eligible[0].id, config.connections[0].id.as_str());
    }
}
