use top200_rs::web::App;
use console_log;

fn main() {
    console_log::init_with_level(log::Level::Debug).unwrap();
    dioxus_web::launch(App);
}
