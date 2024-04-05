use log::{info, log_enabled, Level};
use clap::Parser;
use std::fs;
use std::error::{Error};

mod instrument;
use instrument::{instrument_module, destroy_instrument_module};

use wamr_rust_sdk::{
    runtime::Runtime, module::Module, instance::Instance,
};


#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Instrumentation Scheme
    #[arg(short, long, default_value_t = String::from("empty"))]
    scheme: String,

    /// Instrumentation Arguments
    #[arg(short, long, num_args = 0..)]
    instargs: Vec<String>,

    /// Program arguments 
    #[arg(short, long, num_args = 0..)]
    progargs: Vec<String>, 
    
    /// Output program (instrumented) path
    #[arg(short, long)]
    outfile: Option<String>,

    /// Input program path
    infile: String,
}


fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = CLI::parse();
    let contents = fs::read(&cli.infile)?;
    let args: Vec<&str> = cli.instargs.iter().map(|s| s.as_str()).collect();

    let out_module: &[u8] = instrument_module(contents, cli.scheme.as_str(), &args[..])?;
    if log_enabled!(Level::Debug) {
        let outfile = cli.outfile.expect("Outfile is required for running in debug level");
        info!("Writing module to {}", outfile);
        fs::write(outfile, out_module)?;
    }

    /* WAMR Instantiate and Run */
    let runtime = Runtime::new()?;
    let module = Module::from_buf(&runtime, out_module, "test-module")?;
    let instance = Instance::new(&runtime, &module, 1024 * 256)?;

    let _ = instance.execute_main(&cli.progargs)?;

    info!("Successful execution of wasm");

    destroy_instrument_module(out_module);

    return Ok(());
}
