#![cfg(test)]

use super::RemoteNodeContext;

impl RemoteNodeContext {
    pub(crate) fn poison_resource_admission_for_tests(&self) {
        self.resource_admission.poison_for_tests();
    }
}
