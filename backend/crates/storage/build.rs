//! `sqlx::migrate!` embeds `migrations/` at compile time, but cargo does not
//! know that, so adding a migration file would not rebuild this crate and the
//! new migration would silently be missing. Watching the directory fixes that
//! (the build script `sqlx migrate build-script` generates).

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
