fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    vergen::EmitBuilder::builder()
        .build_timestamp()
        .git_branch()
        .git_commit_timestamp()
        .git_sha(false)
        .rustc_channel()
        .rustc_commit_date()
        .rustc_commit_hash()
        .rustc_host_triple()
        .rustc_llvm_version()
        .rustc_semver()
        .cargo_target_triple()
        .cargo_debug()
        .cargo_opt_level()
        .emit()
        .unwrap();
}
