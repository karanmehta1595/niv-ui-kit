# niv-ui-kit

Swift-style, easy-to-use UI kit for Rust, built on top of egui. Ye **egui wala hissa** hai
(text, colors, theme, aage button/panel/glass). Window/app chalane wala hissa `mainkit` hai
(jaise egui ke saath eframe).

```toml
[dependencies]
mainkit = "0.1.0"
niv-ui-kit = { version = "0.1.0", features = ["all_loaders"] }
```

**Apni alag `egui` dependency mat jodo.** Egui chahiye to `use niv_ui_kit::egui;`.

## Features

- `all_loaders`: image/svg loaders ek saath
- `egui-0-37` (default): egui 0.37
- `egui-0-36`: naam reserved, abhi supported nahi

## Rules

- Zero warnings (`#![deny(warnings)]`).
- Jo cheez niv-ui-kit mein nahi hai, wo egui se chalti hai. `EGUI_GAPS.md` mein list hai.
