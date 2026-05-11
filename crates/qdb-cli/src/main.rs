//! QumuloDB CLI — `qdb`
//!
//! Runs kernel computations from JSON input files and writes JSON results.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(
    name = "qdb",
    about = "QumuloDB kernel CLI",
    version,
    long_about = None
)]
struct Cli {
    /// Data directory for persistence (default: ./qdb_data)
    #[arg(long, default_value = "qdb_data")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run CPM scheduling kernel on a JSON ScheduleRequest
    Schedule {
        /// Path to ScheduleRequest JSON file (- for stdin)
        #[arg(short, long, default_value = "-")]
        input: String,
        /// Output file (- for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,
        /// Persist result to data directory
        #[arg(long)]
        persist: bool,
    },
    /// Run EVM calculation kernel on a JSON EVMRequest
    Evm {
        #[arg(short, long, default_value = "-")]
        input: String,
        #[arg(short, long, default_value = "-")]
        output: String,
        #[arg(long)]
        persist: bool,
    },
    /// Run Monte Carlo simulation on a JSON SimulationRequest
    Simulate {
        #[arg(short, long, default_value = "-")]
        input: String,
        #[arg(short, long, default_value = "-")]
        output: String,
        #[arg(long)]
        persist: bool,
    },
    /// Run resource loading/levelling on a JSON ResourceRequest
    Resources {
        #[arg(short, long, default_value = "-")]
        input: String,
        #[arg(short, long, default_value = "-")]
        output: String,
        #[arg(long)]
        persist: bool,
    },
    /// Read latest persisted result for a project
    Get {
        /// `schedule` | `evm` | `simulation` | `resources`
        table: String,
        project_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let store = qdb_store::QDBStore::open(&cli.data_dir)?;

    match cli.command {
        Commands::Schedule { input, output, persist } => {
            let req: qdb_contract::ScheduleRequest =
                serde_json::from_str(&read_input(&input)?)?;
            let result = qdb_kernel::engines::schedule::run(req)?;
            if persist {
                store.save_schedule(&result)?;
            }
            write_output(&output, &result)?;
        }

        Commands::Evm { input, output, persist } => {
            let req: qdb_contract::EVMRequest =
                serde_json::from_str(&read_input(&input)?)?;
            let result = qdb_kernel::engines::evm::run(req)?;
            if persist {
                store.save_evm(&result)?;
            }
            write_output(&output, &result)?;
        }

        Commands::Simulate { input, output, persist } => {
            let req: qdb_contract::SimulationRequest =
                serde_json::from_str(&read_input(&input)?)?;
            let result = qdb_kernel::engines::simulation::run(req)?;
            if persist {
                store.save_simulation(&result)?;
            }
            write_output(&output, &result)?;
        }

        Commands::Resources { input, output, persist } => {
            let req: qdb_contract::ResourceRequest =
                serde_json::from_str(&read_input(&input)?)?;
            let result = qdb_kernel::engines::resources::run(req)?;
            if persist {
                store.save_resources(&result)?;
            }
            write_output(&output, &result)?;
        }

        Commands::Get { table, project_id } => {
            let json = match table.as_str() {
                "schedule" => store
                    .latest_schedule(&project_id)?
                    .map(|r| serde_json::to_string_pretty(&r).unwrap()),
                "evm" => store
                    .latest_evm(&project_id)?
                    .map(|r| serde_json::to_string_pretty(&r).unwrap()),
                "simulation" => store
                    .latest_simulation(&project_id)?
                    .map(|r| serde_json::to_string_pretty(&r).unwrap()),
                "resources" => store
                    .latest_resources(&project_id)?
                    .map(|r| serde_json::to_string_pretty(&r).unwrap()),
                _ => {
                    eprintln!("unknown table '{}' — use: schedule, evm, simulation, resources", table);
                    std::process::exit(1);
                }
            };
            match json {
                Some(j) => println!("{j}"),
                None => {
                    eprintln!("no data for project '{project_id}' in table '{table}'");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn read_input(path: &str) -> anyhow::Result<String> {
    if path == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

fn write_output(path: &str, value: &impl serde::Serialize) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    if path == "-" {
        println!("{json}");
    } else {
        std::fs::write(path, json)?;
    }
    Ok(())
}
