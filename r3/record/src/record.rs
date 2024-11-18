//! Binary crate for recording a Wasm modules execution and generate a Trace
use clap::Parser;
use libc::c_void;
use log::{info, warn};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult};
use sha256::digest;
use std::error::Error;
use std::fs;
use std::process;

use wamr_rust_sdk::{instance::Instance, module::Module, runtime::Runtime};

use wamr_rust_sdk::{log_level_t, LOG_LEVEL_WARNING};

use common::instrument::{destroy_instrument_module, instrument_module, InstrumentArgs};

pub mod record_interface;
use record_interface::{
    dump_global_trace, initialize_tmpfile_name, wasm_call_tracedump, wasm_memop_tracedump,
};

/// Command-Line Arguments
#[derive(Parser, Debug)]
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

impl CLI {
    /// Print the CLI configuration
    fn print(&self) {
        info!("Scheme: {}", self.scheme);
        info!("Instrumentation Arguments: {:?}", self.instargs);
        info!("Input Command: {:?}", self.input_command);
        info!("Instfile [optional]: {:?}", self.instfile);
        info!("Outfile: {:?}", self.outfile);
    }
}

/// Entrypoint for `record`
pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder().format_timestamp_millis().init();

    let cli = CLI::parse();
    cli.print();

    // Read wasm file, compute its digest
    let infile = cli.input_command[0].as_str();
    let contents = fs::read(infile)?;
    let sha256_infile = digest(&contents);

    let args: Vec<&str> = cli.instargs.iter().map(|s| s.as_str()).collect();
    let inst_module: &[u8] = instrument_module(
        &contents,
        cli.scheme.as_str(),
        InstrumentArgs::Generic(&args[..]),
    )?;
    if let Some(instfile) = cli.instfile {
        info!("Writing module to {}", instfile);
        fs::write(instfile, inst_module)?;
    }

    // This needs to be done before fork to prevent double initialization of
    // Lazy
    initialize_tmpfile_name();
    match unsafe { fork() }? {
        ForkResult::Child => {
            info!("Wasm engine executing with PID: {}", process::id());
            // WAMR Instantiate and Run
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
            info!("Wasm module safely exited from child process");
            process::exit(0);
        }
        ForkResult::Parent { child } => match waitpid(child, None)? {
            WaitStatus::Exited(pid, status) => {
                info!("Wasm engine (PID: {}) exited with status: {}", pid, status);
            }
            status => {
                warn!("Wasm engine exited with bad status: {:?}", status);
            }
        },
    }

    dump_global_trace(&cli.outfile, sha256_infile.as_str())?;
    info!("Dumped trace to {}", cli.outfile);

    destroy_instrument_module(inst_module);

    return Ok(());
}
