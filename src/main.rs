
use chuck_decoder::{ChuckDecoder, DecodingGraph};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "chuck_decoder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Stim {
        #[arg(short, long)]
        file: PathBuf,
        
        /// Path to the Topology JSON file
        #[arg(short, long)]
        graph: PathBuf,
        
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Stim { file, graph, output } => run_stim_integration(file, graph, output),
    }
}

fn run_stim_integration(file_path: PathBuf, graph_path: PathBuf, output_path: Option<PathBuf>) {
    // 1. Load the Graph Map from disk
    let graph = match DecodingGraph::from_file(&graph_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to load graph file: {}", e);
            return;
        }
    };
    
    // 2. Initialize Weights (Default 1%, specific outliers can be loaded from file later)
    let mut weights = HashMap::new();
    for i in 0..100 { weights.insert(i, 0.01); } 
    weights.insert(4, 0.06); // Keep our "Killer Qubit" assumption for now

    let mut decoder = ChuckDecoder::new(weights, graph);
    
    if !file_path.exists() {
        println!("ERROR: Stim file not found at {:?}", file_path);
        return;
    }

    let file = File::open(file_path).expect("Could not open Stim file");
    let reader = io::BufReader::new(file);

    let mut writer = if let Some(path) = output_path {
        Some(io::BufWriter::new(File::create(path).expect("Could not create output file")))
    } else {
        None
    };

    let start = Instant::now();
    let mut lines_processed = 0;
    
    for line in reader.lines() {
        if let Ok(syndrome_raw) = line {
            let correction = decoder.decode(&syndrome_raw);
            if let Some(w) = &mut writer {
                writeln!(w, "{}", correction).unwrap();
            }
            lines_processed += 1;
        }
    }
    let duration = start.elapsed();
    
    println!("Throughput: {:.0} shots/sec", lines_processed as f64 / duration.as_secs_f64());
    println!("Cache Hits: {}", decoder.cache_hits);
}
