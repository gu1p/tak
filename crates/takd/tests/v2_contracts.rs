macro_rules! test_module {
    ($name:ident) => {
        mod $name {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/",
                stringify!($name),
                ".rs"
            ));
        }
    };
}

include!("v2_contracts/attempt_modules.rs");
include!("v2_contracts/worker_modules.rs");
include!("v2_contracts/run_store_modules.rs");
include!("v2_contracts/scheduling.rs");
