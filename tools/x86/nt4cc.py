"""What tools/nt4-clang-cl and tools/x86/nt4-gcc share: C for x86 Windows NT 4.0 with
clang, the same LLVM as rustc's, run on the host (no Wine).

  - Code for a Pentium without MMX or SSE, as the Rust target (NT 4.0 never enables
    SSE), against NT 4.0's C run-time, msvcrt.dll (cl's -MD), without /GS's security
    cookie, which NT 4.0's msvcrt.dll lacks.
  - Headers: ruststdwinnt4's C99 layer (crt/include), then clang's own, then NT 4.0's
    from PINT's SDK (its include and include/crt are NT 4.0's, for every architecture).
    NT4_WINAPI=winsdk: the Windows API headers of the Windows SDK installed with Microsoft
    C for ARM (PINT_MSVC: um, shared and its C++ library's include) for _WIN32_WINNT
    0x0400 in C11 mode, as tools/sdk/nt4-gcc.
  - Libraries: target/implib/x86 (tools/mkimplibs.sh x86: NT 4.0's import libraries,
    oldnames.lib) and the C99 layer's target/crt/x86/nt4c99.lib (crt/build.sh x86).

PINT_SDK and PINT_MSVC come from the environment or nt4.env, as for tools/pint-sdk.
NT4_CLANG names clang (default: clang-18, else clang).
"""

import glob
import os
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TARGET = "i686-pc-windows-msvc"


def settings():
    env = {}
    try:
        with open(os.path.join(ROOT, "nt4.env")) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, v = line.split("=", 1)
                    env[k.strip()] = v.strip().strip("'\"")
    except OSError:
        pass
    get = lambda k, d: os.environ.get(k) or env.get(k) or d
    return (get("PINT_SDK", os.path.join(ROOT, "target", "pint40sdk")),
            get("PINT_MSVC", os.path.join(ROOT, "target", "msvc-arm32")))


def clang():
    c = os.environ.get("NT4_CLANG") or shutil.which("clang-18") or shutil.which("clang")
    if not c:
        sys.exit("nt4cc: no clang (set NT4_CLANG)")
    return c


def crt_include():
    return os.path.join(ROOT, "crt", "include")


def system_includes():
    """The system header directories, in order, after clang's own."""
    sdk, msvc = settings()
    inc = os.path.join(sdk, "rootfs", "opt", "pint40sdk", "include")
    if not os.path.isdir(inc):
        sys.exit(f"nt4cc: no NT 4.0 headers in {inc} (tools/pint-sdk fetch; PINT_SDK)")
    if os.environ.get("NT4_WINAPI") == "winsdk":
        kits = sorted(glob.glob(os.path.join(msvc, "kits", "Include", "*")))
        vc = sorted(glob.glob(os.path.join(msvc, "VC", "Tools", "MSVC", "*", "include")))
        if not kits or not vc:
            sys.exit(f"nt4cc: NT4_WINAPI=winsdk needs the Windows SDK in {msvc} (PINT_MSVC)")
        return [os.path.join(inc, "crt")] + case_folded(
            [os.path.join(kits[-1], "um"), os.path.join(kits[-1], "shared"), vc[-1]])
    return [os.path.join(inc, "crt"), inc]


def case_folded(srcs):
    """The Windows SDK's headers are written for a file system that ignores case
    (Windows.h, included as windows.h; um's headers include shared's SpecStrings.h as
    specstrings.h and the other way round): for each directory, one under
    target/winsdk-ci with a link to each header by its own name, its name in lower case and
    each spelling the headers' #include lines use."""
    import re
    outs = [os.path.join(ROOT, "target", "winsdk-ci", s.strip("/").replace("/", "_")) for s in srcs]
    stamp = os.path.join(ROOT, "target", "winsdk-ci", ".done-" + "+".join(os.path.basename(o) for o in outs))
    if os.path.exists(stamp):
        return outs
    inc = re.compile(rb'^\s*#\s*include\s*[<"]([^>"]+)[>"]', re.M)
    spellings = set()
    for src, out in zip(srcs, outs):
        for dirpath, _, files in os.walk(src):
            rel = os.path.relpath(dirpath, src)
            for d in {rel, rel.lower()}:
                os.makedirs(os.path.join(out, d), exist_ok=True)
            for f in files:
                with open(os.path.join(dirpath, f), "rb") as fh:
                    spellings.update(m.decode(errors="replace").replace("\\", "/") for m in inc.findall(fh.read()))
                for name in {os.path.join(rel, f), os.path.join(rel, f).lower()}:
                    link = os.path.normpath(os.path.join(out, name))
                    if not os.path.lexists(link):
                        os.symlink(os.path.join(dirpath, f), link)
    for out in outs:
        for name in spellings:
            lower = os.path.join(out, name.lower())
            link = os.path.normpath(os.path.join(out, name))
            if os.path.lexists(lower) and not os.path.lexists(link) and os.path.isdir(os.path.dirname(link)):
                os.symlink(os.path.realpath(lower), link)
    open(stamp, "w").close()
    return outs


def early_includes():
    """System header directories ahead of clang's own. NT4_WINAPI=winsdk: an io.h that is
    NT 4.0's and then vcruntime.h, as today's io.h (through corecrt.h) is: code written
    for today's headers takes intptr_t from there (gnulib's fcntl.c). With cl on ARM,
    winnt.h brings vcruntime.h in through VC's intrin.h; on x86 nothing does."""
    if os.environ.get("NT4_WINAPI") != "winsdk":
        return []
    d = os.path.join(ROOT, "target", "winsdk-ci", "shim")
    os.makedirs(d, exist_ok=True)
    for old in ("intrin.h",):
        try:
            os.unlink(os.path.join(d, old))
        except FileNotFoundError:
            pass
    f = os.path.join(d, "io.h")
    text = "/* tools/x86/nt4cc.py: NT 4.0's io.h, then vcruntime.h as today's has. */\n" \
           "#include_next <io.h>\n#include <vcruntime.h>\n"
    if not os.path.exists(f) or open(f).read() != text:
        with open(f, "w") as fh:
            fh.write(text)
    return [d]


def winsdk_defines():
    if os.environ.get("NT4_WINAPI") == "winsdk":
        return ["-D_WIN32_WINNT=0x0400", "-DWINVER=0x0400"]
    return []


# What cl only warns about and clang rejects: code built with PINT's cl on ARM depends on
# it (GMP calls snprintf, which NT 4.0's stdio.h does not declare).
LENIENT = ["-Wno-error=implicit-function-declaration", "-Wno-error=implicit-int",
           "-Wno-error=int-conversion", "-Wno-error=incompatible-function-pointer-types"]


def libdirs():
    return [os.path.join(ROOT, "target", "implib", "x86"), os.path.join(ROOT, "target", "crt", "x86")]


def lld_link():
    """rust-lld of the pinned toolchain as lld-link (its name picks the flavour), for
    clang's driver: target/tools/lld-link."""
    d = os.path.join(ROOT, "target", "tools")
    link = os.path.join(d, "lld-link")
    if not os.path.exists(link):
        import subprocess
        sysroot = subprocess.run(["rustc", "--print", "sysroot"], cwd=ROOT, capture_output=True,
                                 text=True, check=True).stdout.strip()
        found = glob.glob(os.path.join(sysroot, "lib", "rustlib", "*", "bin", "rust-lld"))
        if not found:
            sys.exit("nt4cc: no rust-lld in the toolchain")
        os.makedirs(d, exist_ok=True)
        try:
            os.symlink(found[0], link)
        except FileExistsError:
            pass
    return link
