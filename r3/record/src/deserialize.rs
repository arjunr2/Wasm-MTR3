use log::info;
use clap::Parser;
use std::fs;
use std::error::{Error};
use std::io::{self, Write};

use common::trace::TraceData;

#[derive(Parser,Debug)]
#[command(version, about, long_about=None)]
struct CLI {
    /// Output (modified) Wasm replay file
    #[arg(short, long, default_value_t = String::from("trace.ds"))]
    outfile: String,

    /// Trace output file generated by `record`
    #[arg(default_value_t = String::from("trace.r3"))]
    tracefile: String,
}

fn print_cli(cli: &CLI) {
    info!("Tracefile: {:?}", cli.tracefile);
    info!("Outfile: {:?}", cli.outfile);
}

fn dump_deserialized(deserialized: &TraceData, deserfile: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(deserfile)?;
    for traceop in deserialized.trace.iter() {
        writeln!(file, "{}", traceop)?;
    }
    info!("Deserialized output written to \"{}\"", deserfile);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    
    let cli = CLI::parse();
    print_cli(&cli);

    // Read trace file
    let tracebin = fs::read(cli.tracefile.as_str())?;

    // Don't check for sha256 match; this executable purely deserializes
    let deserialized = TraceData::deserialize(&tracebin, None);

    dump_deserialized(&deserialized, cli.outfile.as_str())?;

    Ok(())
}