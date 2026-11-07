fn main() {
    // SQLx embeds migration files; adding a new file must rebuild the binary.
    println!("cargo:rerun-if-changed=migrations");
}
