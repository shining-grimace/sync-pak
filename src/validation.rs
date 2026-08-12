pub(crate) const NAME_MAX_LENGTH: usize = 60;

pub(crate) fn validate_name_length(value: &str, label: &str) -> Result<(), String> {
    (value.chars().count() <= NAME_MAX_LENGTH)
        .then_some(())
        .ok_or_else(|| format!("{label} must not exceed {NAME_MAX_LENGTH} characters"))
}
