import os, sys, json, tempfile, traceback
null = os.devnull if hasattr(os, "devnull") else ("nul" if os.name == "nt" else "/dev/null")
rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
os.dup2(os.open(null, os.O_RDONLY), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
g = {"__name__": "__main__"}
capture_dir = os.environ.pop("MEI_CAPTURE_DIR", None)
os.environ.pop("MEI_CAPTURE_CREATED", None)
for line in rx:
    code = json.loads(line.decode("utf-8"))
    if not isinstance(code, str):
        raise TypeError("control frame must be a JSON string")
    f = tempfile.NamedTemporaryFile("w+b", delete=False, dir=capture_dir)
    os.dup2(f.fileno(), 1)
    os.dup2(f.fileno(), 2)
    try:
        exec(code, g)
    except BaseException as e:
        traceback.print_exception(type(e), e, e.__traceback__.tb_next)
    finally:
        sys.stdout.flush()
        sys.stderr.flush()
    name = os.path.basename(f.name)
    f.close()
    tx.write(json.dumps(name).encode("ascii") + b"\n")
    tx.flush()
