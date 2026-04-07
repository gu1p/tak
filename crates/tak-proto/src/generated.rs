pub mod tak {
    pub mod proto {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/tak.proto.v1.rs"));
        }
    }
}
