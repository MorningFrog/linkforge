use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};

#[derive(Debug, Parser)]
#[command(name = "linkforge")]
#[command(about = "Create and inspect symbolic links and hard links")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Create a symbolic link to a file or directory")]
    Symlink {
        #[arg(help = "Existing file or directory that the symbolic link points to")]
        source: PathBuf,
        #[arg(help = "Path where the symbolic link will be created")]
        link: PathBuf,
        #[arg(
            long,
            help = "Replace an existing file or symbolic link at the destination"
        )]
        force: bool,
    },
    #[command(about = "Create a hard link to a file")]
    Hardlink {
        #[arg(help = "Existing file that the hard link points to")]
        source: PathBuf,
        #[arg(help = "Path where the hard link will be created")]
        link: PathBuf,
        #[arg(
            long,
            help = "Replace an existing file or symbolic link at the destination"
        )]
        force: bool,
    },
    #[command(about = "Check whether two paths point to the same file")]
    SameFile {
        #[arg(help = "First path to compare")]
        path_a: PathBuf,
        #[arg(help = "Second path to compare")]
        path_b: PathBuf,
    },
    #[command(about = "Show the hard link count for a file")]
    LinkCount {
        #[arg(help = "File whose hard link count should be shown")]
        path: PathBuf,
    },
    #[command(about = "Show sibling paths that are hard links to the same file")]
    Siblings {
        #[arg(help = "File whose hard link siblings should be shown")]
        path: PathBuf,
        #[arg(
            long,
            help = "Directory tree to scan for siblings on platforms that need it"
        )]
        root: Option<PathBuf>,
    },
    #[command(about = "Scan a directory tree for hard link groups")]
    ScanGroups {
        #[arg(help = "Directory tree to scan")]
        root: PathBuf,
    },
    #[command(about = "Clone a directory tree while preserving hard link relationships")]
    CloneTree {
        #[arg(help = "Source directory tree to clone")]
        source_dir: PathBuf,
        #[arg(help = "Destination directory to create")]
        dest_dir: PathBuf,
        #[arg(
            long,
            help = "Replace an existing file or symbolic link at the destination"
        )]
        force: bool,
    },
    #[command(about = "Generate shell completion scripts")]
    Completions {
        #[arg(value_enum, help = "Shell to generate completions for")]
        shell: CompletionShell,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompletionShell {
    #[value(name = "powershell")]
    PowerShell,
    Bash,
    Zsh,
    Fish,
}

impl From<CompletionShell> for Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> std::io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Symlink {
            source,
            link,
            force,
        } => {
            linkforge_core::create_symlink(&source, &link, force)?;
            println!(
                "Created symbolic link: {} -> {}",
                link.display(),
                source.display()
            );
        }
        Command::Hardlink {
            source,
            link,
            force,
        } => {
            linkforge_core::create_hard_link(&source, &link, force)?;
            println!(
                "Created hard link: {} -> {}",
                link.display(),
                source.display()
            );
        }
        Command::SameFile { path_a, path_b } => {
            let same = linkforge_core::is_same_file(&path_a, &path_b)?;
            if same {
                println!("Same file: {} and {}", path_a.display(), path_b.display());
            } else {
                println!(
                    "Different files: {} and {}",
                    path_a.display(),
                    path_b.display()
                );
            }
        }
        Command::LinkCount { path } => {
            let count = linkforge_core::hard_link_count(&path)?;
            println!("Link count for {}: {count}", path.display());
        }
        Command::Siblings { path, root } => {
            let siblings = linkforge_core::hard_link_siblings(&path, root.as_deref())?;
            println!("Hard link siblings for {}:", path.display());
            for sibling in siblings {
                println!("{}", sibling.display());
            }
        }
        Command::ScanGroups { root } => {
            let groups = linkforge_core::scan_hard_link_groups(&root)?;
            if groups.is_empty() {
                println!("No hard link groups found under {}", root.display());
            } else {
                println!("Hard link groups under {}:", root.display());
                for (index, group) in groups.iter().enumerate() {
                    println!("Group {}:", index + 1);
                    for path in &group.paths {
                        println!("{}", path.display());
                    }
                }
            }
        }
        Command::CloneTree {
            source_dir,
            dest_dir,
            force,
        } => {
            linkforge_core::clone_tree_preserve_hardlinks(&source_dir, &dest_dir, force)?;
            println!(
                "Cloned directory tree: {} -> {}",
                source_dir.display(),
                dest_dir.display()
            );
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            generate(
                Shell::from(shell),
                &mut command,
                "linkforge",
                &mut std::io::stdout(),
            );
        }
    }

    Ok(())
}
