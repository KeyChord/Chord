use typeshare::typeshare;

#[typeshare(serialized_as = "String")]
pub type FilePathslug = std::path::PathBuf;
