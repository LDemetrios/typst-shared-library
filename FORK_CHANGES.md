# Fork Changes

Summary of changes from the upstream Typst repository.

- Added `typst-shared` crate. It contains all the glue between Rust's side and any other side, most importantly, c-style
  `extern` functions for calling methods from the compiler (parse syntax, eval, compile, query), and from Typstyle (
  format source)
- Implemented `IntoValue` and `FromValue` for various types. 
- Added parallel serialization mechanism, supporting majority of values (without falling back to `repr`)
- Modified values to better track how they were created from Typst:
    - `tiling`s now preserve their `body` fields
    - `set`-rules either preserve the original value, or try to reconstruct one:
        - `set` method on elements is changed to `set_internal`, which doesn't keep the value -- it marks settings,
          not originating from Typst code, but from other rules, mostly, to my understanding, from realization
        - `#[parse]` clauses are changed for a lot of settable fields
        - A clone of `Derived` was created, with different behaviour for FromValue/IntoValue
    - It is possible to access the closure's text
- Some methods were adapted to WASM target:
    - Scanning fonts has changed
    - Downloading packages now uses https-requests via host function, not TLS-related functions from WASI
- Some fields are not internal anymore: `ContextElem.func`, `Counter.update`, `State.key`, `State.init`,
    `StateUpdate.update`, `FracElem.num_deparenthesized`, `FracElem.denom_deparenthesized`, 