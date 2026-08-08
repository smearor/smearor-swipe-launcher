/// Validate that an instance ID contains only safe characters.
///
/// Only alphanumeric characters, hyphens, and underscores are allowed.
pub fn validate_instance_id(instance_id: &str) -> Result<(), String> {
    if instance_id.is_empty() {
        return Err("Instance ID must not be empty".to_string());
    }
    for ch in instance_id.chars() {
        if !ch.is_alphanumeric() && ch != '-' && ch != '_' {
            return Err(format!(
                "Instance ID '{}' contains invalid character '{}'. Only alphanumeric, hyphen, and underscore are allowed.",
                instance_id, ch
            ));
        }
    }
    Ok(())
}
