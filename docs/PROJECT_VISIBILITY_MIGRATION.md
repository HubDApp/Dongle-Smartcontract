# Project Visibility — Migration Guide (#874)

Projects gain a `visibility: ProjectVisibility` field:

| Mode         | Who can read it                                 | In `list_projects` |
|--------------|-------------------------------------------------|--------------------|
| `Public`     | everyone (`get_project`)                        | yes                |
| `Private`    | owner and contract admins                       | no                 |
| `Restricted` | owner, listed maintainers, and contract admins  | no                 |

- `get_project(project_id)` is an anonymous read and returns `None` for
  non-public projects.
- `get_project_as_viewer(project_id, viewer)` requires `viewer` auth and applies
  the table above.
- `set_project_visibility(project_id, caller, visibility)` is owner-only and
  emits `ProjectVisibilityChanged` (topics `("PROJECT", "VISIBLE", project_id)`,
  data `{ project_id, caller, old_visibility, new_visibility, timestamp }`).
  Setting the current value is a no-op and emits nothing.

## Existing projects

All projects registered before this change are, and remain, **Public**: newly
registered projects default to `Public`, so no action is required from owners
who want to keep the current behaviour.

Because `visibility` is a new field of the stored `Project` struct, `Project`
records written by a previous contract version do not contain it. Before (or as
part of) the upgrade, operators must rewrite each stored project with
`visibility = Public`, following the export → re-import procedure in
[`PROJECT_DATA_MIGRATION.md`](PROJECT_DATA_MIGRATION.md) and
[`STORAGE_MIGRATION_GUIDE.md`](STORAGE_MIGRATION_GUIDE.md):

1. Export every project (`list_projects` / `export_project_data`).
2. Upgrade the contract WASM.
3. Re-persist each project with `visibility: Public`.
4. Verify `get_project(id)` returns every previously public project.

## Integrator changes

- Indexers and frontends that need non-public projects must call
  `get_project_as_viewer` with an authenticated viewer.
- Subscribe to `ProjectVisibilityChanged` to drop projects from public indexes
  when they become `Private` or `Restricted`.
