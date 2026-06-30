use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "linkforge")]
#[command(about = "Create and inspect symbolic links and hard links")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Symlink {
        source: PathBuf,
        link: PathBuf,
        #[arg(long)]
        force: bool,
    },
    Hardlink {
        source: PathBuf,
        link: PathBuf,
        #[arg(long)]
        force: bool,
    },
    SameFile {
        path_a: PathBuf,
        path_b: PathBuf,
    },
    LinkCount {
        path: PathBuf,
    },
    Siblings {
        path: PathBuf,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    ScanGroups {
        root: PathBuf,
    },
    CloneTree {
        source_dir: PathBuf,
        dest_dir: PathBuf,
        #[arg(long)]
        force: bool,
    },
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
    }

    Ok(())
}
