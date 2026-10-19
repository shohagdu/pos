// `sqlx::migrate!` embeds the migrations at compile time; without this, adding
// a new migration file does not trigger a rebuild and it is silently skipped.
fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
