fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("resolve vendored protoc");
    let proto_dir = std::path::PathBuf::from("proto");
    let proto = proto_dir.join("takd.proto");

    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc);
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config
        .compile_protos(&[proto], &[proto_dir])
        .expect("compile proto");
}
