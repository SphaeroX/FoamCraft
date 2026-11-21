// SPDX-License-Identifier: GPL-3.0-or-later
//! `foamcraft`: headless access to FoamCraft.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod api_cmd;
mod jobs_cmd;

use clap::{Parser, Subcommand, ValueEnum};
use foamcraft_core::error::{FoamError, Language};
use foamcraft_runtime::report::{render_check, render_error};
use foamcraft_runtime::{DockerRunner, LogLine, Runner, StartOptions};

#[derive(Parser)]
#[command(name = "foamcraft", version = foamcraft_core::VERSION, about = "FoamCraft command line")]
struct Cli {
    /// Language of messages.
    #[arg(long, value_enum, global = true, default_value = "en")]
    lang: Lang,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Lang {
    En,
    De,
}

impl From<Lang> for Language {
    fn from(l: Lang) -> Self {
        match l {
            Lang::En => Language::En,
            Lang::De => Language::De,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Manage the OpenFOAM runtime (container).
    Runtime {
        #[command(subcommand)]
        action: RuntimeAction,
    },
    /// Mesh and solve the built-in reference case (a cube in a wind tunnel).
    Run {
        /// Solver iterations.
        #[arg(long, default_value_t = 200)]
        iterations: u32,
        /// Upper limit for parallel processes (default: physical cores).
        #[arg(long)]
        cores: Option<u32>,
        /// Folder for job records and logs.
        #[arg(long)]
        jobs_dir: Option<PathBuf>,
    },
    /// Inspect and continue jobs.
    Job {
        #[command(subcommand)]
        action: JobAction,
    },
    /// Create, open and inspect projects.
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// List, describe and run commands of the command API.
    #[command(name = "command")]
    Api {
        #[command(subcommand)]
        action: CommandAction,
    },
    /// Serve the MCP protocol over stdio for AI agents.
    Mcp {
        /// Folder the agent may read and write (projects, models, images); repeat for more.
        #[arg(long = "root", required = true)]
        roots: Vec<std::path::PathBuf>,
        /// Run the commands in the running FoamCraft app, where the user follows them.
        #[arg(long)]
        app: bool,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Create a project in a folder.
    New {
        path: PathBuf,
        /// Project name (default: the folder name).
        #[arg(long)]
        name: Option<String>,
    },
    /// Open a project and print its description.
    Open { path: PathBuf },
    /// Print the description of a project as JSON.
    Info { path: PathBuf },
}

#[derive(Subcommand)]
enum CommandAction {
    /// List all commands.
    List,
    /// Print the description and JSON schemas of a command.
    Describe { name: String },
    /// Run several commands in one session: a JSON list of {"name", "params"}.
    Batch {
        commands: String,
        /// Open this project first.
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Run a command with JSON parameters.
    Run {
        name: String,
        /// Parameters as a JSON object (default: {}).
        params: Option<String>,
        /// Open this project before running the command.
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum JobAction {
    /// List all jobs.
    List {
        #[arg(long)]
        jobs_dir: Option<PathBuf>,
    },
    /// Print the full log of a job.
    Log {
        id: String,
        #[arg(long)]
        jobs_dir: Option<PathBuf>,
    },
    /// Continue following a job that is still running (for example after a restart).
    Resume {
        id: String,
        #[arg(long)]
        jobs_dir: Option<PathBuf>,
    },
    /// Remove the container and the volume of a job (record and log stay).
    Clean {
        id: String,
        #[arg(long)]
        jobs_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum RuntimeAction {
    /// Check Docker and the runtime image; report OpenFOAM version, cores, memory and GPU.
    Check,
    /// Build the runtime image from runtime/Dockerfile.
    Build {
        /// Repository root that contains runtime/Dockerfile.
        #[arg(long)]
        context: Option<PathBuf>,
    },
    /// Stop all containers FoamCraft started.
    Stop,
}

/// Finds the repository root (folder with runtime/Dockerfile) from the current directory upwards,
/// falling back to the source checkout this binary was built from.
fn find_context() -> PathBuf {
    let has_dockerfile = |p: &Path| p.join("runtime").join("Dockerfile").is_file();
    if let Ok(cwd) = std::env::current_dir()
        && let Some(found) = cwd.ancestors().find(|p| has_dockerfile(p))
    {
        return found.to_path_buf();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn runtime_check(lang: Language) -> Result<(), FoamError> {
    let runner = DockerRunner::local();
    let env = runner.check()?;
    if !runner.image_present()? {
        return Err(FoamError::new(
            "runtime.image_missing",
            foamcraft_runtime::IMAGE_TAG,
        ));
    }
    let container = runner.start(&StartOptions::default())?;
    let status = runner.agent_status(&container);
    let _ = runner.stop(&container);
    let status = status?;
    let gpu = status.gpu.available || runner.gpu_passthrough();
    println!("{}", render_check(&env, &status, gpu, lang));
    Ok(())
}

fn runtime_build(context: Option<PathBuf>) -> Result<(), FoamError> {
    let runner = DockerRunner::local();
    runner.check()?;
    let context = context.unwrap_or_else(find_context);
    let mut stdout = std::io::stdout();
    runner.build_image(&context, &mut |line| {
        let text = match line {
            LogLine::Stdout(s) | LogLine::Stderr(s) => s,
        };
        let _ = writeln!(stdout, "{text}");
    })?;
    println!("{}", foamcraft_runtime::IMAGE_TAG);
    Ok(())
}

fn runtime_stop() -> Result<(), FoamError> {
    let n = DockerRunner::local().stop_all()?;
    println!("{n}");
    Ok(())
}

/// The MCP server on stdin/stdout; log lines go to stderr only.
fn mcp(roots: Vec<std::path::PathBuf>, app: bool) -> Result<(), FoamError> {
    use foamcraft_mcp::{McpHandler, Server};
    use rmcp::ServiceExt;
    let roots: Vec<_> = roots
        .into_iter()
        .map(|r| std::path::absolute(&r).unwrap_or(r))
        .collect();
    let data = foamcraft_core::paths::data_dir();
    let server = if app {
        Server::app(&data, roots)?
    } else {
        Server::local(foamcraft_api::registry(), data, roots)
    };
    let failed = |e: String| FoamError::new("mcp.failed", e);
    let rt = tokio::runtime::Runtime::new().map_err(|e| failed(e.to_string()))?;
    rt.block_on(async {
        let running = McpHandler::new(std::sync::Arc::new(server))
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| failed(e.to_string()))?;
        running.waiting().await.map_err(|e| failed(e.to_string()))?;
        Ok(())
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let lang: Language = cli.lang.into();
    let result = match cli.command {
        None => Ok(()),
        Some(Command::Runtime { action }) => match action {
            RuntimeAction::Check => runtime_check(lang),
            RuntimeAction::Build { context } => runtime_build(context),
            RuntimeAction::Stop => runtime_stop(),
        },
        Some(Command::Run {
            iterations,
            cores,
            jobs_dir,
        }) => jobs_cmd::run(iterations, cores, jobs_dir, lang),
        Some(Command::Job { action }) => match action {
            JobAction::List { jobs_dir } => jobs_cmd::list(jobs_dir),
            JobAction::Log { id, jobs_dir } => jobs_cmd::log(&id, jobs_dir),
            JobAction::Resume { id, jobs_dir } => jobs_cmd::resume(&id, jobs_dir, lang),
            JobAction::Clean { id, jobs_dir } => jobs_cmd::clean(&id, jobs_dir),
        },
        Some(Command::Project { action }) => match action {
            ProjectAction::New { path, name } => api_cmd::project_new(&path, name),
            ProjectAction::Open { path } => api_cmd::project_open(&path),
            ProjectAction::Info { path } => api_cmd::project_info(&path),
        },
        Some(Command::Mcp { roots, app }) => mcp(roots, app),
        Some(Command::Api { action }) => match action {
            CommandAction::List => api_cmd::command_list(),
            CommandAction::Describe { name } => api_cmd::command_describe(&name),
            CommandAction::Batch { commands, project } => {
                api_cmd::command_batch(&commands, project.as_deref())
            }
            CommandAction::Run {
                name,
                params,
                project,
            } => api_cmd::command_run(&name, params.as_deref(), project.as_deref()),
        },
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let label = match lang {
                Language::En => "Error",
                Language::De => "Fehler",
            };
            eprintln!("{}", render_error(&err, lang, label));
            ExitCode::FAILURE
        }
    }
}
