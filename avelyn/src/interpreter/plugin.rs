// interpreter/plugin.rs — Dynamic plugin loader

use libloading::{Library, Symbol};
use crate::interpreter::Interpreter;

pub struct PluginManager {
    #[allow(dead_code)]
    libraries: Vec<Library>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager { libraries: Vec::new() }
    }

    pub fn load(&mut self, path: &str, interp: &mut Interpreter) -> Result<(), String> {
        interp.capabilities.check_ffi_load()?;
        // SAFETY: Loading arbitrary shared libraries is inherently unsafe.
        let lib = unsafe { Library::new(path).map_err(|e| format!("FFIError: Failed to load library: {}", e))? };

        // Standard plugin entry point: void avelyn_init(Interpreter* interp)
        type InitFn = unsafe extern "C" fn(*mut Interpreter);

        unsafe {
            let init_res: Result<Symbol<InitFn>, _> = lib.get(b"avelyn_init");
            match init_res {
                Ok(init) => {
                    init(interp as *mut Interpreter);
                }
                Err(e) => {
                    return Err(format!("FFIError: Entry point 'avelyn_init' not found: {}", e));
                }
            }
        }

        self.libraries.push(lib);
        Ok(())
    }
}
