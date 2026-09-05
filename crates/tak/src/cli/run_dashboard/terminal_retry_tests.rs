use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Result, anyhow};

use super::terminal::restore_or_retain;

struct DisplayHarness {
    terminal: Option<()>,
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl DisplayHarness {
    fn restore(&mut self) -> Result<()> {
        let calls = Rc::clone(&self.calls);
        restore_or_retain(&mut self.terminal, move |_| {
            let mut calls = calls.borrow_mut();
            calls.push("show cursor");
            calls.push("leave screen");
            if calls.len() == 2 {
                return Err(anyhow!("temporary terminal restoration failure"));
            }
            Ok(())
        })
    }
}

impl Drop for DisplayHarness {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[test]
fn failed_explicit_restoration_retains_the_terminal_for_drop_to_retry() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    {
        let mut display = DisplayHarness {
            terminal: Some(()),
            calls: Rc::clone(&calls),
        };
        assert!(display.restore().is_err());
        assert!(display.terminal.is_some());
    }

    assert_eq!(
        *calls.borrow(),
        ["show cursor", "leave screen", "show cursor", "leave screen"]
    );
}
