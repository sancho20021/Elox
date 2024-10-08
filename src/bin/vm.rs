extern crate elox;

use qcell::QCellOwner;

use crate::elox::runner::EloxFileAndPromptRunner;
use crate::elox::vm::EloxVM;

fn main() {
    let mut token = QCellOwner::new();
    let mut vm = EloxVM::new();
    if let Err(err) = vm.run_from_std_args(&mut token) {
        println!("{}", err);
    }
}
