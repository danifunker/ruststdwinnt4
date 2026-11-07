//! `__imp_` pointers: how a function NT 4.0 lacks is given to the code that imports it.
//!
//! A call to an imported function goes through a pointer the import library names
//! `__imp_<name>` (`__imp__<name>@<bytes>` on x86); the loader fills it in. For a function
//! NT 4.0 does not export, no import library defines that name, so the pointer is defined
//! here, already holding the address of the implementation.

/// `provide!(Name = implementation, x86 argument bytes);`
macro_rules! provide {
    ($($name:ident = $f:path, $bytes:literal;)*) => {
        $(
            const _: () = {
                #[cfg_attr(not(target_arch = "x86"), export_name = concat!("__imp_", stringify!($name)))]
                #[cfg_attr(target_arch = "x86", export_name = concat!("__imp__", stringify!($name), "@", stringify!($bytes)))]
                #[used]
                static IMP: unsafe extern "system" fn() = unsafe {
                    core::mem::transmute::<*const (), unsafe extern "system" fn()>($f as *const ())
                };
            };
        )*
    };
}
