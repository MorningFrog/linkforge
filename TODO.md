# TODO

## Core Feature Scope

- [x] Create symbolic links for files and directories.
- [x] Create hard links for files.
- [x] Check whether two paths point to the same underlying file.
- [x] Show a file's hard link count.
- [x] Show sibling paths that are hard links to the same file.
  - Linux note: this may require scanning selected filesystem trees because inode-to-path reverse lookup is not generally available as a direct filesystem operation.
- [x] Scan a directory tree to find hard link groups.
- [x] Clone a directory tree while preserving hard link relationships.
