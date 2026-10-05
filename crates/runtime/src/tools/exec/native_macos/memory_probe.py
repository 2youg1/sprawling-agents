"""Disposable macOS runner evidence; no production dependency or daily-host writes."""
import ctypes
import json
import os
import platform
import subprocess
import sys

if platform.system() != "Darwin" or os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("run only inside a disposable GitHub macOS runner")

if "--tree-fixture" in sys.argv:
    child = """import json,os,resource,sys
if sys.argv[1] == 'detached': os.setsid()
pages=bytearray(64*1024*1024)
for offset in range(0,len(pages),4096): pages[offset]=1
print(json.dumps({'ready':True,'allocated_bytes':len(pages),'maxrss':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'detached':sys.argv[1]=='detached'}),flush=True)
sys.stdin.read(1)
"""
    children = []
    records = []
    try:
        for mode in ["ordinary", "detached"]:
            process = subprocess.Popen([sys.executable, "-c", child, mode], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            children.append(process)
            line = process.stdout.readline()
            if not line:
                raise RuntimeError("child allocation failed: " + process.stderr.read())
            records.append(json.loads(line))
        print(json.dumps({"two_children_survived_simultaneously": True, "allocated_total_bytes": sum(row["allocated_bytes"] for row in records), "children": records}), flush=True)
    finally:
        for process in children:
            if process.poll() is None:
                process.stdin.write("x")
                process.stdin.flush()
            process.communicate(timeout=5)
    raise SystemExit(0)

report = {"os": platform.mac_ver()[0], "machine": platform.machine(), "euid": os.geteuid(), "source_commit": os.environ.get("GITHUB_SHA"), "candidate_state": "evidence_only_not_aggregate_enforcement"}
libc = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
# apple-oss-distributions/xnu f6217f...: osfmk/mach/coalition.h lines 43-54,
# 79-80, 162-164 and bsd/kern/syscalls.master lines 703-705.
report["coalition_attempts"] = []
for name, flags in [("resource", 0), ("jetsam", 16)]:
    cid = ctypes.c_uint64(0)
    create = getattr(libc, "coalition_create", None)
    ctypes.set_errno(0)
    if create is None:
        result = libc.syscall(ctypes.c_long(458), ctypes.c_uint32(1), ctypes.byref(cid), ctypes.c_uint32(flags))
        entry = "syscall(458,COALITION_OP_CREATE,cid,typeflags)"
    else:
        create.argtypes = [ctypes.POINTER(ctypes.c_uint64), ctypes.c_uint32]
        create.restype = ctypes.c_int
        result = create(ctypes.byref(cid), ctypes.c_uint32(flags))
        entry = "coalition_create"
    error = ctypes.get_errno()
    row = {"type": name, "entry": entry, "result": result, "errno": error, "error": os.strerror(error) if error else None}
    if result == 0:
        cleanup = []
        for operation in [2, 3]:
            ctypes.set_errno(0)
            status = libc.syscall(ctypes.c_long(458), ctypes.c_uint32(operation), ctypes.byref(cid), ctypes.c_uint32(0))
            cleanup.append({"operation": operation, "result": status, "errno": ctypes.get_errno()})
        row["owned_empty_coalition_cleanup"] = cleanup
    report["coalition_attempts"].append(row)
report["coalition_logical_writes_setter_present"] = getattr(libc, "coalition_ledger_set_logical_writes_limit", None) is not None

for candidate, command in [
    ("taskpolicy_two_children_96MiB", ["/usr/sbin/taskpolicy", "-m", "96", sys.executable, __file__, "--tree-fixture"]),
    ("host_two_children_control", [sys.executable, __file__, "--tree-fixture"]),
]:
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=45)
        report[candidate] = {"returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
    except (OSError, subprocess.TimeoutExpired) as error:
        report[candidate] = {"failed_attempt": type(error).__name__, "detail": str(error)}
print(json.dumps(report, indent=2))
