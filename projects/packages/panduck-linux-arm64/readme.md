# @notedge/panduck-linux-arm64

Native Node-API binary for Panduck on **Linux aarch64** (ARM64 servers, many SBCs, ARM cloud instances).

`@notedge/panduck` selects this optional dependency when `process.platform === 'linux'` and `process.arch === 'arm64'`.
Application projects should still list only `@notedge/panduck`; npm wires the matching `.node` file during install.

If bindings fail to load, compare `uname -m` with the published optional packages and rerun install without omitting
optional dependencies.

Main package: https://www.npmjs.com/package/@notedge/panduck

License: MPL-2.0
