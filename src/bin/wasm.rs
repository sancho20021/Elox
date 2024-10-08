extern crate elox;

// use elox::vm::wasm_target::WasmTarget;
use elox::runner::EloxFileAndPromptRunner;
use elox::vm::target::WasmTarget;
use qcell::QCellOwner;

fn main() {
    let mut token = QCellOwner::new();
    let mut wasm = WasmTarget::new();
    if let Err(err) = wasm.run_from_std_args(&mut token) {
        println!("{}", err);
    }
}
