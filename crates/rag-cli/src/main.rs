use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rag")]
#[command(about = "RAG CLI for local LLM interaction")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Parser, Debug)]
enum Command {
    Ingest {
        #[arg(help = "Path to documents")]
        path: String,
    },
    Query {
        #[arg(help = "Query text")]
        query: String,
    },
    Chat {
        #[arg(help = "Initial message")]
        message: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("RAG CLI - placeholder");
    Ok(())
}
