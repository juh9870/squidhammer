# Changelog

<!-- next-header -->

## [Unreleased]

> Released on ReleaseDate

### Added

- Context to warnings emitted upon loading a file (by juh9870)

### Fixed

- Crash when file names contain wide UTF characters before the extension (by juh9870)

## [0.2.2]

> Released on 2025-02-17

### Added

- More nodes and node search to the node input drag-out menu (by juh9870)
- Types section for add node menu (by juh9870)
- Scrolling to node search (by juh9870)
- Scroll bar to add node menus for very large sections (by juh9870)
- Node selection menu for dragging out wire from output ports (by juh9870)
- Node values inlining (by juh9870)
- Subgraph categories, so they would show up in add node menu (by juh9870)

### Changed

- Wire style to rounded Axis Aligned. This provides clearer picture for when the output wire is conected to the output to the left of the output port (by juh9870)
- Better error reporting when deserializing node graphs (by juh9870)
- Type titles are now shown in search bar (by juh9870)
- Added min height to the undo history scrollbar (by juh9870)

## [0.2.1]

> Released on 2025-02-05

### Added

- `enum_variant_name` node (by juh9870)
- Creation of files inside nested folders (put `/` into the file name when creating a new file or a graph) (by juh9870)
- Better search bar matching and sorting (by juh9870)
- `list_concat` node (by juh9870)
- Assertion nodes: `assert`, `assert_not`, `assert_equals`, `assert_not_equals` (by juh9870)
- `enum_variant_tag` node (by juh9870)
- (internal) Default value capabilities for functional nodes (by juh9870)
- `assert` node (by juh9870)
- `set_mapping` node (by juh9870)
- Subgraphs in node search (by juh9870)
- Transistent storage nodes (`set_transistent_value`, `has_transistent_value`, `try_get_transistent_value`,
  `get_transistent_value`) (by juh9870)
- Duplicate node button (by juh9870)
- `enum_inner_value` node (by juh9870)
- `bool_invert` node (by juh9870)
- `list_contains` node (by juh9870)
- `any_equals` and `any_not_equals` nodes (by juh9870)
- Tag for hiding fields in struct editor (by juh9870)
- List nodes: `list_get`, `list_try_get`, `list_remove`, `list_swap`, `list_move`, `list_rotate_left`,
  `list_rotate_right` (by juh9870)

### Changed

- Sort docs tab entries alphabetically (by juh9870)
- Moved `raw` nodes form `optional` to `utilities` category (by juh9870)
- Updated visual of list editor handle to use dots instead of lines (by juh9870)

### Fixed

- List nodes failing to deserialize in some cases (by juh9870)
- Bad placement of enum editor body (by juh9870)
- Various issues with text editor (by juh9870)

## [0.2.0]

> Released on 2025-02-02

### Breaking Changes

- New module system replaces the old approach, where each type has its own folder. Due to this, all custom types have to
  be re-packaged as dbemodules.
    - `sys` and `color` modules are now bundled indo the editor and can not be modified or removed.
    - `eh` module is now archived and can be downloaded from the artifacts.

---

### Added

- Module system for types (by juh9870)

## [0.1.10]

> Released on 2025-01-26

### Added

- `is_none` node (by juh9870)
- `is_some` node (by juh9870)

### Changed

- `try_set_field` and `set_field` are now geeneric over the object (by juh9870)

### Fixed

- Duplicate ID in docs UI if the input and output fields have the same type (by juh9870)
- `set_field` creating fields with invalid types (by juh9870)

### Documented

- `try_set_field` node (by juh9870)
- `set_field` node (by juh9870)

## [0.1.9]

> Released on 2025-01-26

### Added

- Autosave (by juh9870)
- Saving prompt upon trying to exist an app while a project is open (by juh9870)
- Button to add tabs to the sidebars (by juh9870)
- Update checker (by juh9870)
- Settings menu (currently only for disabling exit prompt) (by juh9870)

### Changed

- Sidebar tabs layout now persist across app restart (by juh9870)
- Changelog format and tag naming (by juh9870)
- Reduced the amount of file writes by avoiding writing unchanged files (by juh9870)
- Parallelized project file saving (by juh9870)

## [v0.1.8]

> Released on 2025-01-25

### Added

- `try_get_field` node (by juh9870)
- `set_field` node (by juh9870)
- `try_set_field` node (by juh9870)
- Changelog management tooling (by juh9870)
- Changelog (by juh9870)

### Changed

- `get_field` node now fails instead of returning None in case of a missing field. Use `try_get_field` to achieve the
  old behavior (by juh9870)

## [v0.1.7]

> Released on 2025-01-23

No changelog for this and all previous versions.

<!-- next-url -->
[Unreleased]: https://github.com/juh9870/squidhammer/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/juh9870/squidhammer/compare/v0.2.1...v0.2.2

[0.2.1]: https://github.com/juh9870/squidhammer/compare/v0.2.0...v0.2.1

[0.2.0]: https://github.com/juh9870/squidhammer/compare/v0.1.10...v0.2.0

[0.1.10]: https://github.com/juh9870/squidhammer/compare/v0.1.9...v0.1.10

[0.1.9]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.8...v0.1.9

[v0.1.8]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.7...squidhammer-v0.1.8

[v0.1.7]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.6...squidhammer-v0.1.7

[v0.1.6]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.5...squidhammer-v0.1.6

[v0.1.5]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.4...squidhammer-v0.1.5

[v0.1.4]: https://github.com/juh9870/dbe/compare/squidhammer-v0.1.3...squidhammer-v0.1.4

[v0.1.3]: https://github.com/juh9870/dbe/compare/scrapyard_editor-v0.1.2...squidhammer-v0.1.3

[v0.1.2]: https://github.com/juh9870/dbe/compare/scrapyard_editor-v0.1.1...scrapyard_editor-v0.1.2

[v0.1.1]: https://github.com/juh9870/dbe/compare/scrapyard_editor-v0.1.0...scrapyard_editor-v0.1.1

[v0.1.0]: https://github.com/juh9870/dbe/tree/scrapyard_editor-v0.1.0