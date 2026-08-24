## Overview

The documentation site will use MkDocs Material because it is simple to operate, has strong navigation/search defaults, and works directly with static GitHub Pages hosting. Documentation source files live under the existing `docs/` directory, while configuration lives in `mkdocs.yml` at the repository root.

## Site Structure

The initial information architecture is intentionally small:

- Home: product overview, alpha status, and quick links.
- Getting Started: Docker Compose quick start and first-user registration.
- Concepts: documents, organization, metadata, indexes, and classifiers.
- Administration: configuration, S3-compatible storage, and background jobs.
- Development: local API/UI development notes.
- Reference: current REST API route-family summary.

This mirrors the major UI/API resource areas without requiring screenshots or generated API schemas in the first iteration.

## Deployment

GitHub Actions builds the site on pushes to `main` that affect docs-related files. The workflow uses the official GitHub Pages artifact deployment flow and runs `mkdocs build --strict` so broken links and configuration issues fail CI.

The `docs/CNAME` file sets the custom domain to `autofile.dev`. If a marketing site later uses the apex domain, docs can move to `docs.autofile.dev` by updating DNS, `docs/CNAME`, and `site_url`.

## Dependencies

Docs dependencies are isolated in `requirements-docs.txt` because Autofile does not currently have Python project metadata. This keeps the application dependency graph unchanged.

## Future Work

- Add screenshots after the UI stabilizes further.
- Add generated OpenAPI documentation if the Rust API adopts an OpenAPI generation crate.
- Add installation examples for production deployments beyond local Docker Compose.
