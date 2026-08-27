use typeshare::typeshare;

// typescript(type = "...") doesn't seem to work for some reason
#[typeshare(serialized_as = "any")]
pub type TomlValue = toml::Value;

/// Converts a TOML value into the string form handed to native handlers. Strings pass through
/// unchanged, scalars use their canonical text, and arrays/tables become compact JSON so structured
/// values remain usable. Embedded NUL bytes are rejected because the value crosses a C ABI.
pub fn toml_value_to_native_arg(value: &toml::Value) -> anyhow::Result<String> {
    let text = match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(_) | toml::Value::Table(_) => serde_json::to_string(value)?,
    };
    if text.contains('\0') {
        anyhow::bail!("argument contains an embedded NUL byte");
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_scalars_and_structures() {
        assert_eq!(toml_value_to_native_arg(&toml::Value::String("a b".into())).unwrap(), "a b");
        assert_eq!(toml_value_to_native_arg(&toml::Value::Integer(3)).unwrap(), "3");
        assert_eq!(toml_value_to_native_arg(&toml::Value::Boolean(true)).unwrap(), "true");
        let array = toml::Value::Array(vec![toml::Value::Integer(1), toml::Value::String("x".into())]);
        assert_eq!(toml_value_to_native_arg(&array).unwrap(), r#"[1,"x"]"#);
    }

    #[test]
    fn rejects_nul() {
        assert!(toml_value_to_native_arg(&toml::Value::String("a\0b".into())).is_err());
    }
}
