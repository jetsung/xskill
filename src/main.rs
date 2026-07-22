mod cache;
mod commands;
mod config;
mod git;
mod lock;
mod output;
mod skill_meta;
mod skill_resolver;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "xskill",
    version,
    about = "XSkill — discover, install, manage reusable agent skill packs",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage configured sources
    Sources {
        #[command(subcommand)]
        action: SourcesAction,
    },

    /// List configured platforms
    Platforms {
        /// Show detailed information
        #[arg(short = 'a', long = "all")]
        detailed: bool,
    },

    /// Install specified skill
    Add {
        /// Source name (config name), ORG/REPO (auto-complete GitHub) or full Git URL
        #[arg(short = 'f', long = "from")]
        source: Option<String>,

        /// Skill name (use '*' to install all skills)
        #[arg(short, long)]
        skill: Option<String>,

        /// Install to global directory
        #[arg(short, long)]
        global: bool,

        /// Specify target platform ('*' for all platforms)
        #[arg(short, long)]
        agent: Option<String>,

        /// Shorthand for --skill `'*'` --agent `'*'` (install all skills to all platforms)
        #[arg(short = 'A', long = "all")]
        all: bool,
    },

    /// Remove skill
    Remove {
        /// Skill name (use '*' to remove all skills)
        #[arg(short, long)]
        skill: Option<String>,

        /// Remove from global directory
        #[arg(short, long)]
        global: bool,

        /// Remove from specific platform ('*' for all platforms)
        #[arg(short, long)]
        agent: Option<String>,

        /// Shorthand for --skill `'*'` --agent `'*'`
        #[arg(short = 'A', long = "all")]
        all: bool,
    },

    /// Query/list skills
    Query {
        /// Source name (config name), ORG/REPO or URL
        #[arg(short = 'f', long = "from")]
        source: Option<String>,

        /// Skill name (required, cannot be empty or '*')
        #[arg(short, long)]
        skill: String,
    },

    /// Manage recommended skills sources
    Rec {
        #[command(subcommand)]
        action: RecAction,
    },

    /// Update installed skills
    Update {
        /// Only update global skills
        #[arg(short, long)]
        global: bool,

        /// Specify skill to update ('*' for all)
        #[arg(short, long)]
        skill: Option<String>,
    },

    /// Restore skills from project lock file
    Restore {
        /// Install to global directory
        #[arg(short, long)]
        global: bool,

        /// Specify target platform ('*' for all platforms)
        #[arg(short, long)]
        agent: Option<String>,

        /// Preview mode: list skills without installing
        #[arg(short = 'D', long = "dry-run")]
        dry_run: bool,
    },

    /// List installed skills
    List {
        /// List global skills
        #[arg(short, long)]
        global: bool,

        /// Filter by specific platform
        #[arg(short, long)]
        agent: Option<String>,
    },

    /// Manage skills cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Manage configuration
    Config {
        /// Initialize config file with default values
        #[arg(short = 'i', long = "init")]
        init: bool,

        /// Open config file in editor
        #[arg(short = 'e', long = "edit")]
        edit: bool,

        /// Get config value by dot path (e.g. cache.enabled)
        #[arg(short = 'g', long = "get")]
        get: Option<String>,

        /// Set config value (e.g. cache.enabled=true)
        #[arg(short = 's', long = "set")]
        set: Option<String>,
    },

    /// Find and install a skill interactively
    Find {
        /// Source name to filter by
        #[arg(short = 'f', long = "from")]
        source: Option<String>,

        /// Initial filter query
        #[arg(short, long)]
        skill: Option<String>,

        /// Install globally (~/.agents)
        #[arg(short, long)]
        global: bool,
    },

    /// Symlink existing skills to a platform
    Link {
        /// Skill name (use '*' to link all skills)
        #[arg(short, long)]
        skill: Option<String>,

        /// Target platform ('*' for all platforms)
        #[arg(short, long)]
        agent: Option<String>,

        /// Link from global directory (~/.agents/skills)
        #[arg(short, long)]
        global: bool,

        /// Shorthand for --skill `'*'` --agent `'*'`
        #[arg(short = 'A', long = "all")]
        all: bool,
    },

    /// Create a new skill project
    New {
        /// Skill name (used as directory name)
        #[arg(short = 'n', long = "name")]
        name: String,

        /// Skill description
        #[arg(short = 'd', long = "description", default_value = "")]
        description: String,

        /// Template type
        #[arg(short = 't', long = "template", default_value = "basic")]
        template: String,
    },
}

#[derive(Subcommand)]
enum SourcesAction {
    /// List configured sources
    List,

    /// Add a new source
    Add {
        /// Source name (optional, defaults to url when empty)
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Source URL (must start with http:// or https://)
        #[arg(short = 'u', long = "url")]
        url: String,

        /// Source type: git or api (default: git)
        #[arg(short = 't', long = "type", default_value = "git")]
        source_type: String,
    },

    /// Remove a source
    Remove {
        /// Source name to remove
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Source URL to remove
        #[arg(short = 'u', long = "url")]
        url: Option<String>,
    },

    /// Edit an existing source (only name can be changed)
    Edit {
        /// Source name to identify entry
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Source URL to identify entry
        #[arg(short = 'u', long = "url")]
        url: Option<String>,

        /// New name for the source (required, empty string to clear)
        #[arg(short = 'N', long = "new-name")]
        new_name: String,
    },
}

#[derive(Subcommand)]
enum RecAction {
    /// List recommended sources
    List,

    /// Add skills to a recommended source
    Add {
        /// Source name (must exist in sources if --url not provided)
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Source URL (when name exists in sources and url matches, only name is saved)
        #[arg(short = 'u', long = "url")]
        url: Option<String>,

        /// Comma-separated list of skill names (required)
        #[arg(short = 's', long = "skills")]
        skills: String,
    },

    /// Remove a recommended source or specific skills
    Remove {
        /// Source name (used to identify entry, or with -u/-s for specific removal)
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Source URL (when both -n and -u provided, -u takes priority)
        #[arg(short = 'u', long = "url")]
        url: Option<String>,

        /// Comma-separated list of skill names to remove (removes specific skills instead of entire entry)
        #[arg(short = 's', long = "skills")]
        skills: Option<String>,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Clear cached data
    Clear {
        /// Clear specific source only
        #[arg(short = 'f', long = "from")]
        from: Option<String>,
    },

    /// Update cache for sources
    Update {
        /// Update specific source only
        #[arg(short = 'f', long = "from")]
        from: Option<String>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sources { action } => match action {
            SourcesAction::List => commands::sources::run(),
            SourcesAction::Add { name, url, source_type } => {
                commands::sources::run_add(name.as_deref(), &url, &source_type)
            }
            SourcesAction::Remove { name, url } => {
                commands::sources::run_remove(name.as_deref(), url.as_deref())
            }
            SourcesAction::Edit { name, url, new_name } => {
                commands::sources::run_edit(name.as_deref(), url.as_deref(), &new_name)
            }
        },
        Commands::Platforms { detailed } => commands::platforms::run(detailed),
        Commands::Add { source, skill, global, agent, all } => {
            // --all is shorthand for --skill '*' --agent '*'
            let (final_skill, final_agent) = if all {
                ("*".to_string(), Some("*".to_string()))
            } else {
                let skill = skill.ok_or_else(|| anyhow::anyhow!("--skill option is required (or use --all)"))?;
                (skill, agent)
            };
            // -s '*' + -a '*' 时必须指定 -f
            if final_skill == "*" && final_agent.as_deref() == Some("*") && source.is_none() {
                anyhow::bail!("--from is required when using --skill '*' --agent '*' (or --all)");
            }
            commands::add::run(source.as_deref(), &final_skill, global, final_agent.as_deref())
        }
        Commands::Remove { skill, global, agent, all } => {
            // --all is shorthand for --skill '*' --agent '*'
            let (final_skill, final_agent) = if all {
                ("*".to_string(), Some("*".to_string()))
            } else {
                let skill = skill.ok_or_else(|| anyhow::anyhow!("--skill option is required (or use --all)"))?;
                (skill, agent)
            };
            commands::remove::run(&final_skill, global, final_agent.as_deref())
        }
        Commands::Query { source, skill } => {
            commands::query::run(&skill, source.as_deref())
        }
        Commands::Rec { action } => match action {
            RecAction::List => commands::rec::run(),
            RecAction::Add { name, url, skills } => {
                commands::rec::run_add(name.as_deref(), url.as_deref(), &skills)
            }
            RecAction::Remove { name, url, skills } => {
                commands::rec::run_remove(name.as_deref(), url.as_deref(), skills.as_deref())
            }
        },
        Commands::Update { global, skill } => {
            commands::update::run(global, skill.as_deref())
        }
        Commands::Restore { global, agent, dry_run } => {
            commands::restore::run(global, agent.as_deref(), dry_run)
        }
        Commands::List { global, agent } => {
            commands::list::run(global, agent.as_deref())
        }
        Commands::Cache { action } => match action {
            CacheAction::Clear { from } => commands::cache::run_clear(from.as_deref()),
            CacheAction::Update { from } => commands::cache::run_update(from.as_deref()),
        },
        Commands::Config { init, edit, get, set } => {
            if init {
                commands::config::run_init()
            } else if edit {
                commands::config::run_edit()
            } else if let Some(key) = get {
                commands::config::run_get(&key)
            } else if let Some(kv) = set {
                commands::config::run_set(&kv)
            } else {
                println!("{}", "Usage: xskill config --init | --edit | --get <key> | --set <key=value>".dimmed());
                Ok(())
            }
        }
        Commands::Find { source, skill, global } => {
            commands::find::run(skill.as_deref(), source.as_deref(), global)
        }
        Commands::Link { skill, agent, global, all } => {
            let (final_skill, final_agent) = if all {
                ("*".to_string(), Some("*".to_string()))
            } else {
                let skill = skill.ok_or_else(|| anyhow::anyhow!("--skill option is required (or use --all)"))?;
                (skill, agent)
            };
            let agent = final_agent.ok_or_else(|| anyhow::anyhow!("--agent option is required (or use --all)"))?;
            commands::link::run(&final_skill, &agent, global)
        }
        Commands::New { name, description, template } => {
            commands::new::run(&name, &description, &template)
        }
    }
}
