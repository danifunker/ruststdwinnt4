//! File functions of later Windows on NT 4.0, through the native API NT 4.0 has
//! (`NtQueryInformationFile`, `NtSetInformationFile`, `NtQueryDirectoryFile`,
//! `NtQueryObject`).

use core::ffi::c_void;
use core::ptr;

use crate::sys::{self, BOOL, HANDLE, IO_STATUS_BLOCK, UNICODE_STRING};

provide! {
    SetFilePointerEx = set_file_pointer_ex, 20;
    GetFileSizeEx = get_file_size_ex, 8;
    GetFileInformationByHandleEx = get_file_information_by_handle_ex, 16;
    SetFileInformationByHandle = set_file_information_by_handle, 16;
    GetFinalPathNameByHandleW = get_final_path_name_by_handle_w, 16;
    CreateHardLinkW = create_hard_link_w, 12;
    CreateSymbolicLinkW = create_symbolic_link_w, 12;
}

const NO_ERROR: u32 = 0;
const INVALID_SET_FILE_POINTER: u32 = 0xFFFF_FFFF;

/// Windows 2000: a 64-bit `SetFilePointer`.
unsafe extern "system" fn set_file_pointer_ex(h: HANDLE, distance: i64, new: *mut i64, method: u32) -> BOOL {
    let mut high = (distance >> 32) as i32;
    // SAFETY: plain calls; new is the caller's, or null.
    unsafe {
        sys::SetLastError(NO_ERROR);
        let low = sys::SetFilePointer(h, distance as i32, &mut high, method);
        if low == INVALID_SET_FILE_POINTER && sys::GetLastError() != NO_ERROR {
            return 0;
        }
        if !new.is_null() {
            *new = ((high as i64) << 32) | low as i64;
        }
    }
    1
}

/// Windows 2000: a 64-bit `GetFileSize`.
unsafe extern "system" fn get_file_size_ex(h: HANDLE, size: *mut i64) -> BOOL {
    let mut high = 0u32;
    // SAFETY: plain calls; size is the caller's.
    unsafe {
        sys::SetLastError(NO_ERROR);
        let low = sys::GetFileSize(h, &mut high);
        if low == 0xFFFF_FFFF && sys::GetLastError() != NO_ERROR {
            return 0;
        }
        *size = ((high as i64) << 32) | low as i64;
    }
    1
}

// FILE_INFO_BY_HANDLE_CLASS values and the native classes they correspond to.
const FILE_BASIC_INFO: i32 = 0;
const FILE_STANDARD_INFO: i32 = 1;
const FILE_NAME_INFO: i32 = 2;
const FILE_RENAME_INFO: i32 = 3;
const FILE_DISPOSITION_INFO: i32 = 4;
const FILE_ALLOCATION_INFO: i32 = 5;
const FILE_END_OF_FILE_INFO: i32 = 6;
const FILE_ATTRIBUTE_TAG_INFO: i32 = 9;
const FILE_ID_BOTH_DIRECTORY_INFO: i32 = 10;
const FILE_ID_BOTH_DIRECTORY_RESTART_INFO: i32 = 11;

const NT_FILE_BOTH_DIRECTORY_INFORMATION: u32 = 3;
const NT_FILE_BASIC_INFORMATION: u32 = 4;
const NT_FILE_STANDARD_INFORMATION: u32 = 5;
const NT_FILE_NAME_INFORMATION: u32 = 9;
const NT_FILE_RENAME_INFORMATION: u32 = 10;
const NT_FILE_LINK_INFORMATION: u32 = 11;
const NT_FILE_DISPOSITION_INFORMATION: u32 = 13;
const NT_FILE_ALLOCATION_INFORMATION: u32 = 19;
const NT_FILE_END_OF_FILE_INFORMATION: u32 = 20;

/// Size of FILE_BASIC_INFO / FILE_BASIC_INFORMATION (four times, attributes, padding).
const BASIC_SIZE: usize = 40;

unsafe fn query(h: HANDLE, info: *mut c_void, size: u32, class: u32) -> BOOL {
    let mut io = IO_STATUS_BLOCK::new();
    // SAFETY: the caller's buffer holds size bytes.
    let s = unsafe { sys::NtQueryInformationFile(h, &mut io, info, size, class) };
    if sys::nt_success(s) { 1 } else { sys::fail_status(s) }
}

/// Windows Vista: the information classes std asks for.
unsafe extern "system" fn get_file_information_by_handle_ex(
    h: HANDLE,
    class: i32,
    info: *mut c_void,
    size: u32,
) -> BOOL {
    // SAFETY (whole body): info holds size bytes, as the caller says.
    unsafe {
        match class {
            // The same layouts as the native classes.
            FILE_BASIC_INFO => query(h, info, size, NT_FILE_BASIC_INFORMATION),
            FILE_STANDARD_INFO => query(h, info, size, NT_FILE_STANDARD_INFORMATION),
            FILE_NAME_INFO => query(h, info, size, NT_FILE_NAME_INFORMATION),
            // { attributes, reparse tag }: NT 4.0 has no reparse points.
            FILE_ATTRIBUTE_TAG_INFO => {
                if size < 8 {
                    return sys::fail(sys::ERROR_INSUFFICIENT_BUFFER);
                }
                let mut basic = [0u64; BASIC_SIZE / 8];
                if query(h, basic.as_mut_ptr().cast(), BASIC_SIZE as u32, NT_FILE_BASIC_INFORMATION) == 0 {
                    return 0;
                }
                let attributes = *(basic.as_ptr().cast::<u8>().add(32) as *const u32);
                *(info as *mut u32) = attributes;
                *(info as *mut u32).add(1) = 0;
                1
            }
            FILE_ID_BOTH_DIRECTORY_INFO | FILE_ID_BOTH_DIRECTORY_RESTART_INFO => {
                id_both_directory(h, info as *mut u8, size as usize, class == FILE_ID_BOTH_DIRECTORY_RESTART_INFO)
            }
            _ => sys::fail(sys::ERROR_INVALID_PARAMETER),
        }
    }
}

// FILE_BOTH_DIR_INFORMATION and FILE_ID_BOTH_DIR_INFO: the same fields up to ShortName,
// then FileId (8 bytes, 8-aligned) in the latter only, then the name.
const BOTH_NAME_OFFSET: usize = 94;
const ID_FILE_ID_OFFSET: usize = 96;
const ID_NAME_OFFSET: usize = 104;
const NAME_LENGTH_OFFSET: usize = 60;
/// The largest FILE_ID_BOTH_DIR_INFO entry: a 255-character name, aligned.
const ID_MAX_ENTRY: usize = (ID_NAME_OFFSET + 255 * 2 + 7) & !7;

/// FileIdBothDirectoryInfo (Vista, native class 37 of Windows 2000) from NT 4.0's
/// FileBothDirectoryInformation, one entry at a time so none is lost: an entry is read
/// only while the caller's buffer has room for the largest one.
unsafe fn id_both_directory(h: HANDLE, out: *mut u8, size: usize, restart: bool) -> BOOL {
    let mut scratch = [0u64; (BOTH_NAME_OFFSET + 255 * 2 + 7) / 8 + 1];
    let mut used = 0usize;
    let mut last: *mut u32 = ptr::null_mut();
    let mut first = true;
    while size - used >= ID_MAX_ENTRY {
        let mut io = IO_STATUS_BLOCK::new();
        // SAFETY: scratch is ours; the handle is the caller's directory.
        let s = unsafe {
            sys::NtQueryDirectoryFile(
                h, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut io,
                scratch.as_mut_ptr().cast(), (scratch.len() * 8) as u32,
                NT_FILE_BOTH_DIRECTORY_INFORMATION, 1, ptr::null(), (restart && first) as u8,
            )
        };
        first = false;
        if s == sys::STATUS_NO_MORE_FILES {
            break;
        }
        if !sys::nt_success(s) {
            if used == 0 {
                return sys::fail_status(s);
            }
            break;
        }
        // SAFETY: one entry in scratch; the copy fits (checked above).
        unsafe {
            let src = scratch.as_ptr().cast::<u8>();
            let name_len = *(src.add(NAME_LENGTH_OFFSET) as *const u32) as usize;
            let dst = out.add(used);
            ptr::copy_nonoverlapping(src, dst, BOTH_NAME_OFFSET);
            ptr::write_bytes(dst.add(BOTH_NAME_OFFSET), 0, ID_NAME_OFFSET - BOTH_NAME_OFFSET);
            let _ = ID_FILE_ID_OFFSET; // FileId: 0, NT 4.0 has none to give.
            ptr::copy_nonoverlapping(src.add(BOTH_NAME_OFFSET), dst.add(ID_NAME_OFFSET), name_len);
            let entry = (ID_NAME_OFFSET + name_len + 7) & !7;
            *(dst as *mut u32) = 0; // NextEntryOffset: the last so far
            if !last.is_null() {
                *last = (dst as usize - last as usize) as u32;
            }
            last = dst as *mut u32;
            used += entry;
        }
    }
    if used == 0 {
        return sys::fail(sys::ERROR_NO_MORE_FILES);
    }
    1
}

const FILE_DISPOSITION_INFO_EX: i32 = 21;
const FILE_RENAME_INFO_EX: i32 = 22;
const ERROR_NOT_SUPPORTED: u32 = 50;

/// Windows Vista: the classes NT 4.0's native API has. Those of later Windows (POSIX
/// delete and rename, FileDispositionInfoEx and FileRenameInfoEx) fail with
/// ERROR_INVALID_PARAMETER, on which std falls back to the older ones.
unsafe extern "system" fn set_file_information_by_handle(
    h: HANDLE,
    class: i32,
    info: *const c_void,
    size: u32,
) -> BOOL {
    let nt = match class {
        FILE_BASIC_INFO => NT_FILE_BASIC_INFORMATION,
        FILE_RENAME_INFO => NT_FILE_RENAME_INFORMATION,
        FILE_DISPOSITION_INFO => NT_FILE_DISPOSITION_INFORMATION,
        FILE_ALLOCATION_INFO => NT_FILE_ALLOCATION_INFORMATION,
        FILE_END_OF_FILE_INFO => NT_FILE_END_OF_FILE_INFORMATION,
        FILE_DISPOSITION_INFO_EX | FILE_RENAME_INFO_EX => return sys::fail(sys::ERROR_INVALID_PARAMETER),
        _ => return sys::fail(ERROR_NOT_SUPPORTED),
    };
    let mut io = IO_STATUS_BLOCK::new();
    // SAFETY: info holds size bytes in the layout of the class, which is the native one.
    let s = unsafe { sys::NtSetInformationFile(h, &mut io, info, size, nt) };
    if sys::nt_success(s) { 1 } else { sys::fail_status(s) }
}

/// Copy `src` into the caller's buffer as GetFinalPathNameByHandleW returns it: the
/// length without the NUL, or the size needed with it when the buffer is too small.
unsafe fn give_path(parts: &[&[u16]], out: *mut u16, out_len: u32) -> u32 {
    let n: usize = parts.iter().map(|p| p.len()).sum();
    if n + 1 > out_len as usize {
        return (n + 1) as u32;
    }
    let mut at = 0;
    for p in parts {
        // SAFETY: out holds out_len characters, checked above.
        unsafe { ptr::copy_nonoverlapping(p.as_ptr(), out.add(at), p.len()) };
        at += p.len();
    }
    // SAFETY: as above.
    unsafe { *out.add(n) = 0 };
    n as u32
}

const VOLUME_NAME_NT: u32 = 2;
const VOLUME_NAME_NONE: u32 = 4;

/// Windows Vista: the path of an open file, from its object name (`\Device\...\path`),
/// with the device replaced by the drive letter it is mapped to (`\\?\C:\path`), or by
/// `\\?\UNC\` for the network redirector. `VOLUME_NAME_NT` gives the object name, and
/// `VOLUME_NAME_NONE` the path on the volume; a GUID volume name (Windows 2000) is not
/// supported.
unsafe extern "system" fn get_final_path_name_by_handle_w(h: HANDLE, out: *mut u16, out_len: u32, flags: u32) -> u32 {
    // OBJECT_NAME_INFORMATION: a UNICODE_STRING, then its characters.
    let mut buf = [0u64; 1100 / 8 * 2];
    let mut ret = 0u32;
    // SAFETY: buf is ours.
    let s = unsafe { sys::NtQueryObject(h, 1, buf.as_mut_ptr().cast(), (buf.len() * 8) as u32, &mut ret) };
    if !sys::nt_success(s) {
        sys::fail_status(s);
        return 0;
    }
    // SAFETY: NtQueryObject filled in the UNICODE_STRING, pointing into buf.
    let name: &[u16] = unsafe {
        let us = &*(buf.as_ptr() as *const UNICODE_STRING);
        core::slice::from_raw_parts(us.buffer, us.length as usize / 2)
    };
    let volume = flags & 7;
    if volume == VOLUME_NAME_NT {
        // SAFETY: the caller's buffer.
        return unsafe { give_path(&[name], out, out_len) };
    }

    const REDIRECTOR: &[u16] = &[
        b'\\' as u16, b'D' as u16, b'e' as u16, b'v' as u16, b'i' as u16, b'c' as u16, b'e' as u16,
        b'\\' as u16, b'L' as u16, b'a' as u16, b'n' as u16, b'm' as u16, b'a' as u16, b'n' as u16,
        b'R' as u16, b'e' as u16, b'd' as u16, b'i' as u16, b'r' as u16, b'e' as u16, b'c' as u16,
        b't' as u16, b'o' as u16, b'r' as u16,
    ];
    const UNC: &[u16] = &[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16, b'U' as u16, b'N' as u16, b'C' as u16];

    if starts_with_ci(name, REDIRECTOR) && name.get(REDIRECTOR.len()) == Some(&(b'\\' as u16)) {
        // \Device\LanmanRedirector[\;X:0]\server\share\path
        let mut rest = &name[REDIRECTOR.len()..];
        if rest.get(1) == Some(&(b';' as u16)) {
            let skip = rest[1..].iter().position(|&c| c == b'\\' as u16).map_or(rest.len(), |p| p + 1);
            rest = &rest[skip..];
        }
        // SAFETY: the caller's buffer.
        return unsafe {
            if volume == VOLUME_NAME_NONE {
                give_path(&[rest], out, out_len)
            } else {
                give_path(&[UNC, rest], out, out_len)
            }
        };
    }

    let mut target = [0u16; 512];
    for letter in b'A'..=b'Z' {
        let drive = [letter as u16, b':' as u16, 0];
        // SAFETY: drive is NUL-terminated; target holds its length.
        let n = unsafe { sys::QueryDosDeviceW(drive.as_ptr(), target.as_mut_ptr(), target.len() as u32) };
        if n == 0 {
            continue;
        }
        let device = &target[..target.iter().position(|&c| c == 0).unwrap_or(0)];
        if !device.is_empty()
            && starts_with_ci(name, device)
            && matches!(name.get(device.len()), None | Some(&0x5C))
        {
            let rest = &name[device.len()..];
            let root = [b'\\' as u16];
            let rest: &[u16] = if rest.is_empty() { &root } else { rest };
            let prefix = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16, letter as u16, b':' as u16];
            // SAFETY: the caller's buffer.
            return unsafe {
                if volume == VOLUME_NAME_NONE {
                    give_path(&[rest], out, out_len)
                } else {
                    give_path(&[&prefix, rest], out, out_len)
                }
            };
        }
    }
    sys::fail(sys::ERROR_PATH_NOT_FOUND);
    0
}

fn starts_with_ci(s: &[u16], prefix: &[u16]) -> bool {
    s.len() >= prefix.len()
        && s.iter().zip(prefix).all(|(&a, &b)| {
            // SAFETY: plain calls.
            unsafe { sys::RtlUpcaseUnicodeChar(a) == sys::RtlUpcaseUnicodeChar(b) }
        })
}

/// NT 4.0 has hard links (on NTFS: FileLinkInformation), only not this function
/// (Windows 2000).
unsafe extern "system" fn create_hard_link_w(new: *const u16, existing: *const u16, _sa: *mut c_void) -> BOOL {
    const FILE_WRITE_ATTRIBUTES: u32 = 0x100;
    const SYNCHRONIZE: u32 = 0x10_0000;
    const SHARE_ALL: u32 = 7;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    // SAFETY (whole body): the caller's NUL-terminated paths; buffers made and freed here.
    unsafe {
        let f = sys::CreateFileW(
            existing, FILE_WRITE_ATTRIBUTES | SYNCHRONIZE, SHARE_ALL, ptr::null_mut(), OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS, ptr::null_mut(),
        );
        if f == sys::INVALID_HANDLE_VALUE {
            return 0;
        }
        let mut nt = UNICODE_STRING { length: 0, maximum_length: 0, buffer: ptr::null_mut() };
        if sys::RtlDosPathNameToNtPathName_U(new, &mut nt, ptr::null_mut(), ptr::null_mut()) == 0 {
            sys::CloseHandle(f);
            return sys::fail(sys::ERROR_PATH_NOT_FOUND);
        }
        // FILE_LINK_INFORMATION: ReplaceIfExists (padded), RootDirectory, FileNameLength,
        // FileName.
        let size = 12 + nt.length as usize;
        let heap = sys::GetProcessHeap();
        let info = sys::HeapAlloc(heap, 0, size) as *mut u8;
        let mut r = 0;
        if !info.is_null() {
            *(info as *mut u32) = 0;
            *(info.add(4) as *mut usize) = 0;
            *(info.add(8) as *mut u32) = nt.length as u32;
            ptr::copy_nonoverlapping(nt.buffer as *const u8, info.add(12), nt.length as usize);
            let mut io = IO_STATUS_BLOCK::new();
            let s = sys::NtSetInformationFile(f, &mut io, info.cast(), size as u32, NT_FILE_LINK_INFORMATION);
            r = if sys::nt_success(s) { 1 } else { sys::fail_status(s) };
            sys::HeapFree(heap, 0, info.cast());
        }
        sys::HeapFree(heap, 0, nt.buffer.cast());
        let e = sys::GetLastError();
        sys::CloseHandle(f);
        sys::SetLastError(e);
        r
    }
}

/// Windows Vista; NT 4.0 has no symbolic links.
unsafe extern "system" fn create_symbolic_link_w(_link: *const u16, _target: *const u16, _flags: u32) -> u8 {
    sys::fail(ERROR_NOT_SUPPORTED);
    0
}
