use log::{info};
use clap::Parser;
use std::fs;
use std::io::{self, Read};
use libc::c_void;
use sha256::digest;
use std::error::{Error};

use common::trace::*;

#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Trace output file generated by `record`
    #[arg(short, long, default_value_t = String::from("trace.r3"))]
    tracefile: String,

    /// Output (modified) Wasm replay file
    #[arg(short, long, default_value_t = String::from("replay.r3"))]
    outfile: String,

    /// Original (unmodified) Wasm file
    #[arg(short, long)]
    wasmfile: String,
}

fn print_cli(cli: &CLI) {
    info!("Wasmfile: {:?}", cli.wasmfile);
    info!("Tracefile: {:?}", cli.tracefile);
    info!("Outfile: {:?}", cli.outfile);
}


fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    let cli = CLI::parse();
    print_cli(&cli);

    // Read wasm file, compute its digest
    let wasmbin = fs::read(cli.wasmfile.as_str())?;
    let sha256_wasm = digest(&wasmbin);
    
    // Read trace file
    let tracebin = fs::read(cli.tracefile.as_str())?;

    let deserialized = TraceDataDeser::deserialize(&tracebin,
         Some(sha256_wasm.as_str()));


    Ok(())
}
