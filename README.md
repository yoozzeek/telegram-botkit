# telegram-botkit

[![CI](https://github.com/yoozzeek/telegram-botkit/actions/workflows/ci.yml/badge.svg)](https://github.com/yoozzeek/telegram-botkit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/telegram-botkit.svg)](https://crates.io/crates/telegram-botkit)
[![Docs.rs](https://docs.rs/telegram-botkit/badge.svg)](https://docs.rs/telegram-botkit)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache2-yellow.svg)](./LICENSE)

Botkit it's minimal, declarative UI kit that you or your AI agent need
for building reliable A+ Telegram bots on Rust. Built on top of Teloxide.

- Declarative scenes (Svelte/React‑vibe): you render views, botkit applies them.
- Tiny router + viewport keep menus interactive (edit/reply/new, prompts) and can persist per‑message meta (Redis).
- Clean session layer for active scene/prompt state.

## Roadmap

- [x] Router core and common helpers
- [x] Viewport with in-memory and redis stores
- [x] UIKit with editors, formatters, callback and message helpers
- [ ] Router composer
- [ ] Guidelines & More examples
- [ ] Run live demo bot

## Quickstart

Add to Cargo.toml:

```toml
[dependencies]
telegram-botkit = { version = "0.1" }
# optional features: "redis", "metrics"
```

## Abstract scene

```rust
struct HelloScene;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum State {
    Root
}

#[derive(Clone, Debug)]
enum Event {}

impl Scene for Hello {
    const VERSION: u16 = 1;
    const ID: &'static str = "hello";
    const PREFIX: &'static str = "h";

    type State = State;
    type Event = Event;

    fn init(&self, _ctx: &Ctx) -> Self::State {
        State::Root
    }

    fn render(&self, _ctx: &Ctx, _s: &Self::State) -> View {
        View {
            text: "Hello".into(),
            markup: None,
            parse_mode: None,
            disable_web_page_preview: None
        }
    }

    fn update(&self, _ctx: &Ctx, s: &Self::State, _e: Self::Event) -> Effect<Self::State> {
        Effect::Stay(s.clone(), RenderPolicy::EditOrReply)
    }

    fn bindings(&self) -> Bindings<Self::Event> {
        Bindings { msg: vec![], cb: vec![] }
    }
}
```

## Examples

* [Simple scene](./examples/simple_scene.rs)
* [Complex scene](./examples/complex_scene.rs)
* [Navigation](./examples/navigation.rs)
* [Redis storage](./examples/redis_storage.rs)
* [Webhook](./examples/webhook.rs)

## Built-in UIKit

### ui::editors

- edit_percent
- edit_percent_positive
- edit_percent_negative
- edit_time_secs
- edit_u64
- edit_u64_valid
- edit_string_nonempty
- edit_lamports
- edit_base58_address
- ok_percent
- ok_time
- ok_lamports

### ui::message

- compact_reply
- refresh_or_reply_with
- clear_input_prompt_message
- clear_input_prompt_message_id
- delete_incoming
- notify_ephemeral
- sanitize_markdown_v2
- ReplyOptions (type)
- EditOptions (type)

### ui::keyboard

- rows
- to_row
- toggles_row
- choice_row
- label_selected
- toggle_label
- selected_label_with
- toggle_icon

### ui::callback

- answer_callback_safe
- show_success_alert
- show_warning_alert
- show_error_alert

### ui::formatters

- parse_percent_to_bp
- parse_time_duration
- parse_u64_or_none
- parse_solana_address
- parse_sol_to_lamports
- format_duration_short
- format_sol

## License

This project is licensed under the Apache 2.0 License. See [LICENSE](./LICENSE) for details.
