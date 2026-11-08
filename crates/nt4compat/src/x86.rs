//! Functions x86 NT 4.0 lacks that other architectures' NT 4.0 has (PINT's NTDLL exports
//! `RtlCaptureContext`; x86 got it with Windows XP).

provide! {
    RtlCaptureContext = rtl_capture_context, 4;
}

/// The caller's registers in an x86 `CONTEXT` (`CONTEXT_CONTROL | CONTEXT_INTEGER`): as
/// they will be when this returns to it. std calls it to start a backtrace.
#[unsafe(naked)]
unsafe extern "system" fn rtl_capture_context(_context: *mut u8) {
    core::arch::naked_asm!(
        "push eax",
        "mov eax, [esp + 8]", // the CONTEXT
        "mov [eax + 0xA4], ebx",
        "mov [eax + 0xAC], ecx",
        "mov [eax + 0xA8], edx",
        "mov [eax + 0xA0], esi",
        "mov [eax + 0x9C], edi",
        "mov [eax + 0xB4], ebp",
        "mov ecx, [esp]", // the caller's eax
        "mov [eax + 0xB0], ecx",
        "mov ecx, [esp + 4]", // the return address
        "mov [eax + 0xB8], ecx",
        "lea ecx, [esp + 12]", // esp once this has returned and popped its argument
        "mov [eax + 0xC4], ecx",
        "pushfd",
        "pop ecx",
        "mov [eax + 0xC0], ecx",
        "xor ecx, ecx",
        "mov cx, cs",
        "mov [eax + 0xBC], ecx",
        "mov cx, ss",
        "mov [eax + 0xC8], ecx",
        "mov dword ptr [eax], 0x10003", // CONTEXT_i386 | CONTEXT_CONTROL | CONTEXT_INTEGER
        "mov ecx, [eax + 0xAC]",
        "pop eax",
        "ret 4",
    )
}
