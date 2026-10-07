# @notedge/panduck-linux-x64

Native Node-API binary for Panduck on **Linux x86_64**.

This tarball exists so `@notedge/panduck` can ship prebuilt conversion speed on typical Linux desktops, CI runners, and
servers without compiling Rust locally. Consumers should depend on `@notedge/panduck`, not on this package name.

After `npm install -g @notedge/panduck`, verify the binding with `panduck doctor`. Conversion examples and route
matrices live on the main package readme.

Main package: https://www.npmjs.com/package/@notedge/panduck

License: MPL-2.0
