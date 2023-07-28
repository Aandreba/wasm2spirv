use wasm2spirv::Compilation;

#[test]
fn test() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let config = serde_json::from_str(include_str!("../examples/cast/cast.json"))?;
    let wasm = wat::parse_bytes(include_bytes!("../examples/cast/cast.wat"))?;
    let compile = Compilation::new(config, &wasm)?;

    println!("{}", compile.spvc_msl()?);
    return Ok(());
}
