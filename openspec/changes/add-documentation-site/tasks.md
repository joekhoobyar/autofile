## 1. OpenSpec

- [x] Initialize OpenSpec for the repository.
- [x] Create the `add-documentation-site` change.
- [x] Add proposal, design, task, and spec artifacts.

## 2. Documentation Site

- [x] Add `mkdocs.yml` configured for MkDocs Material.
- [x] Add `requirements-docs.txt` for docs dependencies.
- [x] Add initial Markdown documentation under `docs/`.
- [x] Add `docs/CNAME` for `autofile.dev`.

## 3. Deployment

- [x] Add a GitHub Pages workflow that deploys from `main`.
- [x] Configure the workflow to build with `mkdocs build --strict`.

## 4. Verification

- [x] Run `openspec validate add-documentation-site --strict`.
- [x] Run `mkdocs build --strict`.
