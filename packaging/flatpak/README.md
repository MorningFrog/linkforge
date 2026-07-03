# Flatpak Draft

App ID: `io.github.morningfrog.LinkForge`

This draft is for local preparation only. Do not open a Flathub submission before explicit 1.0 release approval.

## Sandbox Decision

LinkForge creates links across user-selected filesystem locations. The draft grants broad home and removable-media filesystem access so the native feature set can be tested. Before public Flatpak submission, validate whether portals or a reduced feature set can replace static filesystem permissions.

Flatpak does not install the host GNOME Files `nautilus-python` extension. Users who need file-manager integration should use native deb/rpm packages.

## Preparing Offline Cargo Sources

The draft manifest builds from the local source tree, enables the `org.freedesktop.Sdk.Extension.rust-stable` SDK extension for Cargo/Rust inside the sandboxed build, and requires a vendored Cargo source directory plus Cargo source replacement config before `flatpak-builder` validation:

```text
mkdir -p .cargo
cargo vendor --versioned-dirs vendor > .cargo/config.toml
```

Do not commit the generated `vendor/` directory or local `.cargo/` config; both are ignored for release-preparation work. For public Flatpak or Flathub submission, replace the local `type: dir` source in `io.github.morningfrog.LinkForge.yml` with a release archive source and real SHA256, or adopt generated Flatpak Cargo sources as part of the release artifact workflow.

Real application screenshots are still required before public Flathub submission. The draft MetaInfo intentionally does not reference a missing screenshot file.

The current draft builds, bundles, installs, launches, performs sandboxed file-link operations, and uninstalls in WSLg, but it is not Flathub-ready. The Flathub linter reports `finish-args-home-filesystem-access` for broad home access, and repo lint reports missing AppStream screenshots via `metainfo-missing-screenshots` and `appstream-screenshots-not-mirrored-in-ostree`.

## Local Validation

```text
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak-builder --user --force-clean --install-deps-from=flathub build-dir packaging/flatpak/io.github.morningfrog.LinkForge.yml
flatpak-builder --user --install --force-clean --install-deps-from=flathub build-dir packaging/flatpak/io.github.morningfrog.LinkForge.yml
flatpak run io.github.morningfrog.LinkForge
flatpak install --user -y flathub org.flatpak.Builder
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest packaging/flatpak/io.github.morningfrog.LinkForge.yml
flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo
appstreamcli validate packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml
```

Also test GUI launch, file-link operations under sandbox permissions, and uninstall cleanup.

## Future Flathub Submission

When approved: build locally, run the linter, open the Flathub new-app pull request, respond to review, then maintain updates through pull requests after acceptance.
