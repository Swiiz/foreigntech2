use foreigntech2::app::App;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    App::run();
}
