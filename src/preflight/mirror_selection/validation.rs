use super::{MirrorChoice, MirrorSelection};

impl MirrorSelection {
    pub(super) fn validate(&self) -> Result<(), &'static str> {
        for entry in self.original().comparison() {
            let needs_choice = [
                MirrorChoice::Keep,
                MirrorChoice::Delete,
                MirrorChoice::UseLocal,
                MirrorChoice::UseRemote,
            ]
            .into_iter()
            .any(|choice| self.allowed(entry, choice));
            if needs_choice && self.choice(&entry.path).is_none() {
                return Err(
                    "Assign an action to every eligible file before starting. Choose actions individually or use the section controls.",
                );
            }
            if self.choice(&entry.path) == Some(MirrorChoice::Delete) {
                let prefix = format!("{}/", entry.path.as_str());
                if self.original().comparison().iter().any(|child| {
                    child.path.as_str().starts_with(&prefix)
                        && self.choice(&child.path) != Some(MirrorChoice::Delete)
                }) {
                    return Err("To delete a folder, select Delete for every entry inside it too.");
                }
            }
        }
        Ok(())
    }
}
