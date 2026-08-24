## Why

Autofile needs a public documentation site so users can install, configure, and understand the application without relying on the repository README alone. GitHub Pages and MkDocs Material provide a simple, low-maintenance path for publishing documentation at `https://autofile.dev/` now, with the option to move docs later when a marketing site exists.

## What Changes

- Add a MkDocs Material documentation site sourced from the existing `docs/` directory.
- Publish the documentation site to GitHub Pages from the `main` branch.
- Configure the custom domain `autofile.dev` for the generated site.
- Add initial docs for getting started, core concepts, administration, development, and API reference orientation.
- Document that Autofile is relatively stable, but still alpha software.

## Capabilities

### New Capabilities

- `documentation-site`: Defines the requirements for building, organizing, and deploying the Autofile documentation site.

### Modified Capabilities

- None.

## Impact

- Adds MkDocs configuration and documentation dependency files at the repository root.
- Adds Markdown documentation content under `docs/`.
- Adds a GitHub Actions workflow under `.github/workflows/` for Pages deployment.
- Adds OpenSpec artifacts under `openspec/changes/add-documentation-site/`.
