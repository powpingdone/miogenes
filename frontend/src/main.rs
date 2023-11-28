use mio_glue::MioClientState;
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() {
    let state = Rc::new(RefCell::new(MioClientState::new()));
    TopLevelWindow::new().unwrap().run().unwrap();
}
