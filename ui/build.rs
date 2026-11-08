fn main() {
    // protoc embarqué : pas de dépendance à un protoc système.
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().expect("protoc vendored"));
    let protos = ["studio", "core", "keymap", "behaviors", "meta"].map(|n| format!("proto/{n}.proto"));
    prost_build::compile_protos(&protos, &["proto"]).expect("compilation des .proto");
    println!("cargo:rerun-if-changed=proto");
    tauri_build::build()
}
