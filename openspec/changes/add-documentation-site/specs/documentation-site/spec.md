## ADDED Requirements

### Requirement: MkDocs Material Documentation Site

Autofile SHALL provide a MkDocs Material documentation site whose source content is stored under the repository `docs/` directory.

#### Scenario: Build documentation locally

- **GIVEN** the docs dependencies are installed from `requirements-docs.txt`
- **WHEN** a maintainer runs `mkdocs build --strict` from the repository root
- **THEN** MkDocs builds the site successfully using `docs/` as the content source

### Requirement: Initial Documentation Coverage

The documentation site SHALL include initial pages for getting started, core product concepts, administration, development, and API reference orientation.

#### Scenario: Navigate primary docs sections

- **GIVEN** the documentation site is built
- **WHEN** a user opens the navigation menu
- **THEN** the user can access Getting Started, Concepts, Administration, Development, and Reference sections

### Requirement: Classifier Block Documentation

The documentation site SHALL explain classifier block behavior and fully document the supported classifier rules YAML syntax.

#### Scenario: Understand classifier execution

- **GIVEN** a user needs to configure classifier blocks
- **WHEN** the user reads the classifier block documentation
- **THEN** the user can understand block ordering, matching, actions, child rules, and `continue_after_match` behavior

#### Scenario: Write classifier YAML

- **GIVEN** a user needs to manually edit classifier rules YAML
- **WHEN** the user reads the YAML reference
- **THEN** the user can identify the required top-level fields, pattern syntax, action keys, snippet replacement syntax, and supported modifiers

### Requirement: GitHub Pages Deployment

Autofile SHALL publish the documentation site to GitHub Pages from the `main` branch using GitHub Actions.

#### Scenario: Deploy docs changes

- **GIVEN** a docs-related change is pushed to `main`
- **WHEN** the GitHub Pages workflow runs
- **THEN** the workflow builds the MkDocs site and deploys the generated artifact to GitHub Pages

### Requirement: Custom Domain

The documentation site SHALL be configured for the custom domain `autofile.dev`.

#### Scenario: Preserve custom domain during deploy

- **GIVEN** the site is built for GitHub Pages
- **WHEN** the generated artifact is deployed
- **THEN** the deployment includes a `CNAME` file containing `autofile.dev`
