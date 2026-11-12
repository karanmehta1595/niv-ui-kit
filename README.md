# niv-ui-kit

Swift-style, easy-to-use UI kit for Rust, built on top of egui. Ye **egui wala hissa** hai
(text, colors, theme, button/panel/glass/window). Window/app chalane wala hissa `mainkit` hai
(jaise egui ke saath eframe).

```toml
[dependencies]
mainkit = "0.1.1"
niv-ui-kit = { version = "0.1.1", features = ["all_loaders"] }
```

**Apni alag `egui` dependency mat jodo.** Egui chahiye to `use niv_ui_kit::egui;`.

```rust
use niv_ui_kit::prelude::*;
```

## Features

- `all_loaders`: image/svg loaders ek saath
- `egui-0-37` (default): egui 0.37
- `egui-0-36`: naam reserved, abhi supported nahi

## Rules

- Zero warnings (`#![deny(warnings)]`).
- Jo cheez niv-ui-kit mein nahi hai, wo egui se chalti hai. `EGUI_GAPS.md` mein list hai.

---

## Quick reference

### Text (headings, styled text) — `text.rs`

Dedicated `Heading` widget nahi hai — size + bold chain karke heading banao:

```rust
ui.text("My Note").bold().size(24.0);                 // heading
ui.text("subtitle").size(14.0).opacity(0.6);           // subtitle
ui.text("Hello ").bold().then("world").italic();       // mixed styles, same line
if ui.text("Click me").response().clicked() { /* .. */ }
```

Chain-able: `.bold() .italic() .underline() .strike() .mono() .color(Color) .size(px)
.opacity(0.0-1.0) .selectable() .wrap() .truncate()`. Statement khatam hote hi apne aap
draw ho jata hai — `.response()` sirf jab click/hover chahiye.

Helpers: `fit(ui, text, size, width)` (…-truncate), `wrap_lines(ui, text, size, width)`,
`text_width(ui, text, size, mono)`, `copy_text(ui, text)`.

### Text input / text area — `textfield.rs`, `edit.rs`

Styled box (background, rounded corners, focus ring) — single OR multi-line:

```rust
TextField(&mut title).hint("Title").show(ui);                         // single line
TextField(&mut body).multiline().height(300.0).hint("Write a note..").show(ui); // TEXT AREA
```

`.hint(..) .width(px) .height(px) .radius(px) .text_size(px) .accent(Color) .id(..)
.focus() .multiline()`. `response.changed()` / `.lost_focus()`.

Raw, unstyled editor (apna background banao khud — code editor jaisa use ke liye):

```rust
CodeEdit(&mut body).full_width().min_size(vec2(400.0, 300.0)).show(ui); // multi-line
CodeEdit(&mut name).single_line().show(ui);
```

Cursor/selection helpers: `selection(ctx, id)`, `set_selection(ctx, id, start, end)`,
`caret_ends(ctx, id)`.

### Panel — `panel.rs`

```rust
Panel("left").left().width(240.0).resizable(true).glass().blur(40).show_in(ui, |ui| { .. });
```
`.left()/.right()/.top()/.bottom()/.center() .fill(Color) .fill_opacity(0-100)
.transparent(bool) .frame(None) .range(min, max) .range_full(px) .glass().blur(0-100)`.
`default_dark()/default_light()` = theme defaults; `light(c)/dark(c)` override one theme only.

### Button — `button.rs`

```rust
Button::new("Save").full_width().show(ui);
Button::image("icon_id").label("Open").show(ui);
glass_button("Cancel").show(ui);
```
Corner radius on every state, `.scale(State::push(0.1).clicked())`, `.free()/.claim()`
placement, `Hover`/`Pressed`/`Bundle`/`Locate`/`Label`/`Image` variants.

### Layout — `layout.rs`

```rust
Row::new().gap(8.0).show(ui, |ui| { .. });
Column::new().align(Align::Center).show(ui, |ui| { .. });
```
niv's own names instead of egui's `ui.horizontal`/`Layout` — `.reverse() .gap_start/.gap_end
/.gap_around .left/.center/.right`.

### Group — `group.rs`

Toolbar-capsule style: several `Button`s in one backing rect with separators —
`Group("id").show(ui, |g| { .. })`, per-button `.button_corners("id", tl, tr, br, bl)`.

### Window — `window.rs` / `mac_window.rs`

Floating-in-app, a real separate OS window, OR an attached side panel from one `Window("id")`:
`.separate()`, `.attached(Outside::left().size(..))`.

### Theme / colors — `theme.rs`

```rust
rgb(10, 132, 255); rgba(10, 132, 255, 180); hex("#0A84FF");
dark(Color::BLACK); light(Color::WHITE);     // per-theme override
set_theme(ThemeMode::Dark);
```

### Others

`Switch` (toggle), `Dropdown` (popup list), `Menu`/`MenuLine` (line-box menu),
`Card` (elevated rect), `Shadow`/`drop(x, y)` (drop/inner shadow on Button/Image/Group/Panel/Card),
`Canvas` (free drawing), `Img`/`Load` (image with loader), `Popup`/`Tip` (hover tooltip),
`Scroll`/`Bottom` (scroll area), `Terminal` (real shell widget via `portable-pty`).

---

## Status

Phase 1: layered on egui 0.37 (egui kept inside as the engine + escape hatch for anything
not listed above). A fully standalone, egui-free rebuild (`nivshell` + `nivkit-ui`) is being
built in parallel under new crate names — this crate (niv-ui-kit) stays as the stable,
published version alongside it.
