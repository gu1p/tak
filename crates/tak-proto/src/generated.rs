pub mod tak {
    pub mod proto {
        pub mod v2 {
            include!(concat!(env!("OUT_DIR"), "/tak.proto.v2.rs"));
        }
    }
}
