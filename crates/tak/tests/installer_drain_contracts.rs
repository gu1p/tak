#[path = "support/installer_drain.rs"]
mod installer_drain;

mod release_bundle {
    include!("get_tak_installer_legacy_drain_contract.rs");
}

mod agent {
    include!("get_takd_installer_legacy_drain_contract.rs");
}

mod source {
    include!("local_installer_legacy_drain_contract.rs");
}
