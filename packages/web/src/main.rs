#![allow(clippy::clone_on_copy)]

use dioxus::prelude::*;

mod app;
mod hooks;
mod i18n;
mod router;
mod services;
mod stores;
mod utils;
mod views;

use app::App;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(Root);
}

#[component]
fn Root() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        App {}
    }
}
