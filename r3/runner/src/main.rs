use log::{info};
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

mod interface;
use interface::{wasm_r3_replay_proc_exit, wasm_r3_replay_writev};

#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Log-level within the Wasm engine
    #[arg(short, long, default_value_t = LOG_LEVEL_WARNING)]
    verbose: log_level_t,

    /// Input Command (Wasm program path + Argv) 
    #[arg(num_args = 1..)]
    input_command: Vec<String>,
}

fn print_cli(cli: &CLI) {
    info!("Input Command: {:?}", cli.input_command);
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp_millis()
        .init();

    let cli = CLI::parse();
    print_cli(&cli);

    // Read wasm file, compute its digest
    let infile = cli.input_command[0].as_str();
    let wasm_module = fs::read(infile)?;

    /* WAMR Instantiate and Run */
    let runtime = Runtime::builder()
        .use_system_allocator()
        .set_host_function_module_name("r3-replay")
        .register_host_function("SC_proc_exit", wasm_r3_replay_proc_exit as *mut c_void)
        .register_host_function("SC_writev", wasm_r3_replay_writev as *mut c_void)
        .set_max_thread_num(100)
        .build()?;
    runtime.set_log_level(cli.verbose);
    let module = Module::from_buf(&runtime, &wasm_module[..], infile)?;
    let instance = Instance::new(&runtime, &module, 1024 * 256)?;

    let _ = instance.execute_main(&cli.input_command)?;

    info!("Finished execution of Wasm file");

    return Ok(());
}