use typeshare::typeshare;

// typescript(type = "...") doesn't seem to work for some reason
#[typeshare(serialized_as = "any")]
pub type TomlValue = toml::Value;
