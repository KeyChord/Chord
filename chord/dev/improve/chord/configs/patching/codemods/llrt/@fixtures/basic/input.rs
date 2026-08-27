fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    generate_sdk_client_endpoint_map(&out_dir)?;

    generate_bytecode_cache(&out_dir)?;

    Ok(())
}
