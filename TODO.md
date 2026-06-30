# TODO

## Core Feature Scope

- [ ] Create symbolic links for files and directories.
- [ ] Create hard links for files.
- [ ] Check whether two paths point to the same underlying file.
- [ ] Show a file's hard link count.
- [ ] Show sibling paths that are hard links to the same file.
  - Linux note: this may require scanning selected filesystem trees because inode-to-path reverse lookup is not generally available as a direct filesystem operation.
- [ ] Scan a directory tree to find hard link groups.
- [ ] Clone a directory tree while preserving hard link relationships.
