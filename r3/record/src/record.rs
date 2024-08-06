use log::{info};
use clap::Parser;
use std::fs;
use libc::c_void;
use sha256::digest;
use std::error::{Error};

use wamr_rust_sdk::{
    runtime::Runtime, module::Module, instance::Instance,
};

use wamr_rust_sdk::{
    log_level_t,
    LOG_LEVEL_WARNING
};

use common::instrument::{instrument_module, destroy_instrument_module};

mod tracer;
use tracer::{wasm_memop_tracedump, wasm_call_tracedump, dump_global_trace};


#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Instrumentation Scheme
    #[arg(short, long, default_value_t = String::from("r3-record"))]
    scheme: String,

    /// Instrumentation Arguments
    #[arg(short = 'a', long = "args", num_args = 0..)]
    instargs: Vec<String>,

    /// Log-level within the Wasm engine
    #[arg(short, long, default_value_t = LOG_LEVEL_WARNING)]
    verbose: log_level_t,

    /// Output trace path
    #[arg(short, long, default_value_t = String::from("trace.r3"))]
    outfile: String,

    /// Instrumented program path
    #[arg(short, long)]
    instfile: Option<String>,

    /// Input Command (Wasm program path + Argv) 
    #[arg(num_args = 1..)]
    input_command: Vec<String>,
}

fn print_cli(cli: &CLI) {
    info!("Scheme: {}", cli.scheme);
    info!("Instrumentation Arguments: {:?}", cli.instargs);
    info!("Input Command: {:?}", cli.input_command);
    info!("Instfile [optional]: {:?}", cli.instfile);
    info!("Outfile: {:?}", cli.outfile);
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp_millis()
        .init();

    let cli = CLI::parse();
    print_cli(&cli);

    // Read wasm file, compute its digest
    let infile = cli.input_command[0].as_str();
    let contents = fs::read(infile)?;
    let sha256_infile = digest(&contents);

    let args: Vec<&str> = cli.instargs.iter().map(|s| s.as_str()).collect();
    let inst_module: &[u8] = instrument_module(contents, cli.scheme.as_str(), &args[..])?;
    if let Some(instfile) = cli.instfile {
        info!("Writing module to {}", instfile);
        fs::write(instfile, inst_module)?;
    }

    /* WAMR Instantiate and Run */
    let runtime = Runtime::builder()
        .use_system_allocator()
        .set_host_function_module_name("instrument")
        .register_host_function("memop_tracedump", wasm_memop_tracedump as *mut c_void)
        .register_host_function("call_tracedump", wasm_call_tracedump as *mut c_void)
        .set_max_thread_num(100)
        .build()?;
    runtime.set_log_level(cli.verbose);
    let module = Module::from_buf(&runtime, inst_module, infile)?;
    let instance = Instance::new(&runtime, &module, 1024 * 256)?;

    let _ = instance.execute_main(&cli.input_command)?;

    info!("Successful execution of wasm");

    dump_global_trace(&cli.outfile, sha256_infile.as_str())?;
    info!("Dumped trace to {}", cli.outfile);

    destroy_instrument_module(inst_module);

    return Ok(());
}
