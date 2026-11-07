//! Restart NT at once: the end of a test run on PINT (tests/pint/run-exe.sh). The kernel
//! flushes the file systems and restarts, without asking Winlogon or the programs running.
//! Needs the shutdown privilege, which an administrator has.
#![no_std]
#![no_main]

use nt4rt::eprintln;

nt4rt::entry!(main);

const SE_SHUTDOWN_PRIVILEGE: u32 = 19;
const SHUTDOWN_REBOOT: u32 = 1;

#[link(name = "ntdll")]
extern "system" {
    fn RtlAdjustPrivilege(privilege: u32, enable: u8, client: u8, was_enabled: *mut u8) -> i32;
    fn NtShutdownSystem(action: u32) -> i32;
}

fn main() -> u32 {
    let mut was = 0u8;
    // SAFETY: both take plain values and the one out parameter given.
    unsafe {
        let st = RtlAdjustPrivilege(SE_SHUTDOWN_PRIVILEGE, 1, 0, &mut was);
        if st < 0 {
            eprintln!("reboot: no shutdown privilege (status {:#010x})", st as u32);
            return 1;
        }
        let st = NtShutdownSystem(SHUTDOWN_REBOOT);
        eprintln!("reboot: NtShutdownSystem failed (status {:#010x})", st as u32);
    }
    1
}
