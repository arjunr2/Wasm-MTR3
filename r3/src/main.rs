use log::{info, log_enabled, Level};
use clap::Parser;
use std::fs;
use libc::c_void;
use std::error::{Error};

use wamr_rust_sdk::{
    runtime::Runtime, module::Module, instance::Instance,
};

use wamr_rust_sdk::{
    log_level_t,
    LOG_LEVEL_WARNING
};


mod instrument;
use instrument::{instrument_module, destroy_instrument_module};

mod tracer;
use tracer::{wasm_tracedump};


#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Instrumentation Scheme
    #[arg(short, long, default_value_t = String::from("empty"))]
    scheme: String,

    /// Instrumentation Arguments
    #[arg(short, long, num_args = 0..)]
    instargs: Vec<String>,

    /// Runtime log-level
    #[arg(short, long, default_value_t = LOG_LEVEL_WARNING)]
    verbose: log_level_t,

    /// Output program (instrumented) path
    #[arg(short, long)]
    outfile: Option<String>,

    /// Input Command (Wasm program path + Argv) 
    #[arg(num_args = 1..)]
    input_command: Vec<String>,
}

fn print_cli(cli: &CLI) {
    info!("Scheme: {}", cli.scheme);
    info!("Instrumentation Arguments: {:?}", cli.instargs);
    info!("Input Command: {:?}", cli.input_command);
    info!("Outfile [optional]: {:?}", cli.outfile);
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = CLI::parse();
    print_cli(&cli);
    let infile = cli.input_command[0].as_str();
    let contents = fs::read(infile)?;
    let args: Vec<&str> = cli.instargs.iter().map(|s| s.as_str()).collect();

    let out_module: &[u8] = instrument_module(contents, cli.scheme.as_str(), &args[..])?;
    if log_enabled!(Level::Debug) {
        panic!("Outfile is required for running in debug level");
    }
    if let Some(outfile) = cli.outfile {
        info!("Writing module to {}", outfile);
        fs::write(outfile, out_module)?;
    }

    /* WAMR Instantiate and Run */
    let runtime = Runtime::builder()
        .use_system_allocator()
        .set_host_function_module_name("instrument")
        .register_host_function("tracedump", wasm_tracedump as *mut c_void)
        .set_max_thread_num(20)
        .build()?;
    runtime.set_log_level(cli.verbose);
    let module = Module::from_buf(&runtime, out_module, infile)?;
    let instance = Instance::new(&runtime, &module, 1024 * 256)?;

    let _ = instance.execute_main(&cli.input_command)?;

    info!("Successful execution of wasm");

    destroy_instrument_module(out_module);

    return Ok(());
}
