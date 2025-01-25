- Author: juh9870
- Kind: Changed
---
`get_field` node now fails instead of returning None in case of a missing field. Use `try_get_field` to achieve the old behavior