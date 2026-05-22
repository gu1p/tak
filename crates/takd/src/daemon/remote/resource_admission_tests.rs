#![cfg(test)]

use super::SharedResourceAdmission;

impl SharedResourceAdmission {
    pub(crate) fn poison_for_tests(&self) {
        let inner = self.inner.clone();
        let _ = std::thread::spawn(move || {
            let _guard = inner.state.lock().expect("resource admission lock");
            panic!("poison resource admission");
        })
        .join();
    }
}
