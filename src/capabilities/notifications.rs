use super::CapabilityError;

pub trait DesktopNotifier {
    fn show(&self, notification: &DesktopNotification<'_>) -> Result<(), CapabilityError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopNotification<'a> {
    pub title: &'a str,
    pub body: &'a str,
}
