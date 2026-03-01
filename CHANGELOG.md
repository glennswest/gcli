# Changelog

## [Unreleased]
- **feat:** Podman-based builds on server1 for Linux x86_64, local macOS ARM64 build
- **refactor:** Replace GitHub Actions Linux/macOS builds with server1 podman builds
- **feat:** Build script (`scripts/build.sh`) for multi-platform builds
- **refactor:** GitHub Actions release workflow now only builds Windows binary

## [v3.0.1] — 2026-03-01

## [v3.0.0] — 2026-03-01

### Added
- Initial release with multi-agent chat, voice STT/TTS, PPTX generation, code audit, web search, auto-git, self-update
- README with usage and build instructions
- GitHub Actions release workflow with multi-platform builds (macOS ARM64, Linux x86_64/ARM64, Windows x86_64)
- GitHub Actions CI workflow (cargo check + clippy on PRs)
- Version bump script (`scripts/release.sh`) for automated releases
- CHANGELOG.md for tracking changes

### Fixed
- Resolved all compile errors (string literals, API mismatches, moved values, duplicate imports)
- Version source of truth consolidated to Cargo.toml (removed hardcoded version in main.rs)
- Self-update repo_owner corrected to "glennswest"
