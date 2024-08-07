use log::info;
use clap::Parser;
use std::fs;
use std::error::{Error};
use std::io::{self, Write};
use sha256::digest;

use common::trace::*;

mod parser;
use parser::{dump_replay_ops, construct_replay_ops};

mod generator;
use generator::{generate_replay_file};

mod structs;

#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Trace output file generated by `record`
    #[arg(short, long, default_value_t = String::from("trace.r3"))]
    tracefile: String,

    /// Output (modified) Wasm replay file
    #[arg(short, long, default_value_t = String::from("replay.r3"))]
    outfile: String,

    /// Deserialized debug output file
    #[arg(short, long)]
    debugfile: Option<String>,

    /// Transformed replay operations output file
    #[arg(short, long)]
    replayfile: Option<String>,

    /// Original (unmodified) Wasm file
    #[arg(short, long)]
    wasmfile: String,
}

fn print_cli(cli: &CLI) {
    info!("Wasmfile: {:?}", cli.wasmfile);
    info!("Tracefile: {:?}", cli.tracefile);
    info!("Outfile: {:?}", cli.outfile);
}

pub fn dump_deserialized(deserialized: &TraceDataDeser, debugfile: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(debugfile)?;
    for traceop in deserialized.trace.iter() {
        match traceop {
            TraceOp::MemOp(access) => writeln!(file, "{}", access)?,
            TraceOp::CallOp(call) => writeln!(file, "{}", call)?
        }
    }
    info!("Deserialized output written to {}", debugfile);
    Ok(())
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

    if let Some(debugfile) = cli.debugfile {
        dump_deserialized(&deserialized, debugfile.as_str())?;
    }

    let replay_ops = construct_replay_ops(&deserialized.trace);
    if let Some(replayfile) = cli.replayfile {
        dump_replay_ops(&replay_ops, replayfile.as_str()).unwrap();
    }

    generate_replay_file(&replay_ops, &wasmbin, &cli.outfile)?; 

    Ok(())
}
