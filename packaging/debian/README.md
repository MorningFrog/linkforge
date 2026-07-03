# Debian And Ubuntu PPA Draft

This directory documents the Debian/Ubuntu packaging draft. The actual Debian metadata lives in the repository-root `debian/` directory so standard Debian tools can discover it when run from the source root.

## Channel Decision

Ubuntu PPA is the first practical apt-compatible channel for LinkForge. Official Debian packaging is deferred until there is an ITP/RFP decision, public VCS review, and a Debian Developer sponsor or maintainer path.

## Package Split

- `linkforge`: meta package that installs the full native desktop surface.
- `linkforge-cli`: CLI executable and shell completions.
- `linkforge-gui`: Tauri GUI, desktop file, AppStream metadata, and icon.
- `linkforge-context-menu-gnome`: GNOME Files integration package.

Native Linux packages install the full LinkForge surface by default through the `linkforge` meta package.

The packaged Nautilus extension is staged at `packaging/debian/share/nautilus-python/extensions/linkforge.py` and should be kept in sync with the generated extension in `crates/linkforge-context-menu-gnome`.

## Network-Independent Build Decision

Release source packages must be built from a source tarball that includes vendored Cargo dependencies and Cargo offline config. Package builders must not fetch crates from the network. The draft `debian/rules` uses `--offline` to enforce that expectation.

Before building a source package, prepare a vendored tree:

```text
cargo vendor vendor
```

Then add the generated Cargo source replacement config to the source tarball. The source package build is expected to run from the repository root, where `debian/rules` can see `Cargo.toml`, workspace crates, and `target/`.

## Dependencies

Expected Ubuntu/Debian build dependencies include Rust/Cargo, `pkg-config`, OpenSSL development headers, GTK/WebKitGTK development packages, AppIndicator development packages, librsvg, and `patchelf`.

Expected GNOME integration runtime dependencies include `python3`, `python3-gi`, Nautilus introspection bindings, and the distro package that provides `nautilus-python` integration, commonly `python3-nautilus` on Debian/Ubuntu.

## Local Validation

```text
chmod +x debian/rules
debuild -S -us -uc
sbuild ../linkforge_0.1.0-0ubuntu1.dsc
lintian
desktop-file-validate packaging/flatpak/io.github.morningfrog.LinkForge.desktop
appstreamcli validate packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml
```

Also test install, upgrade, remove, shell completions, GUI launch, CLI launch, and GNOME Files context-menu smoke tests in a clean VM.

## Ubuntu PPA Dry Run

Do not upload before approval. When approved for a private/test PPA only:

1. Create a Launchpad account and GPG key.
2. Build a signed source package.
3. Upload with `dput` to a private/test PPA.
4. Remember that Launchpad builds binary packages from uploaded source packages rather than accepting local binaries.

## Debian Official Path

For official Debian later:

- Prepare an ITP/RFP decision.
- Keep packaging in public VCS.
- Seek review through Debian mentors or an existing Debian Developer sponsor.
- Do not assume direct archive upload rights.
