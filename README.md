# LinkForge

LinkForge is a tool for quickly creating and managing file symbolic links and hard links. It is designed to reduce the cost of manually maintaining links in everyday development and file organization workflows.

## Feature Overview

LinkForge's core feature scope includes:

- Creating symbolic links for files and directories.
- Creating hard links for files.
- Checking whether two paths point to the same underlying file.
- Showing a file's hard link count.
- Showing the sibling paths that are hard links to the same file. On Linux, this may require scanning one or more filesystem trees because filesystems generally do not maintain a direct reverse index from an inode to every pathname that references it.
- Scanning a directory tree to find hard link groups.
- Cloning a directory tree while preserving hard link relationships, so files that were hard-linked in the source remain hard-linked in the clone.

## Platform Support

- Core link management supports Windows and Linux.
- The GUI is designed for Windows desktop environments and the Linux GNOME desktop environment.

## Project Structure

- `crates/linkforge-core`: Core link management logic.
- `crates/linkforge-cli`: Command-line interface entry point.
- `crates/linkforge-gui`: Graphical interface entry point.
