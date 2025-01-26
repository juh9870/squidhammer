# Changelog

<!-- next-header -->

## [Unreleased]

> Released on ReleaseDate

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
[Unreleased]: https://github.com/juh9870/squidhammer/compare/v0.1.10...HEAD
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