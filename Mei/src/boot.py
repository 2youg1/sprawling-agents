import os, sys, json, signal, tempfile, traceback, codecs
rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
os.dup2(os.open(os.devnull, os.O_RDONLY), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
g = {"__name__": "__main__"}
def boom(*_): raise TimeoutError("timeout after 250s")
def snapshot(f, size):
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    head, tail, count = "", "", 0
    left = size
    while left > 0:
        chunk = f.read(min(65536, left))
        if not chunk: break
        left -= len(chunk)
        text = decoder.decode(chunk)
        count += len(text)
        head = (head + text)[:8000]
        tail = (tail + text)[-8000:]
    text = decoder.decode(b"", final=True)
    count += len(text)
    head = (head + text)[:8000]
    tail = (tail + text)[-8000:]
    if count > 16000:
        return (head + f"\n[... truncated; full output: {f.name} ...]\n" + tail, True)
    if count <= 8000: return (head, False)
    return (head + tail[-(count - 8000):], False)
for line in rx:
    code = json.loads(line.decode("utf-8"))
    if not isinstance(code, str): raise TypeError("control frame must be a JSON string")
    f = tempfile.NamedTemporaryFile("w+b", delete=False)
    os.dup2(f.fileno(), 1); os.dup2(f.fileno(), 2)
    signal.signal(signal.SIGALRM, boom)
    signal.alarm(250)
    try: exec(code, g)
    except BaseException as e: traceback.print_exception(type(e), e, e.__traceback__.tb_next)
    finally: signal.alarm(0)
    sys.stdout.flush(); sys.stderr.flush()
    size = os.fstat(f.fileno()).st_size
    name = f.name
    f.close()
    # A fresh handle owns its file offset: a model thread still writing to
    # the shared output offset appends past the bounded snapshot instead of
    # overwriting its head, and the fstat bound keeps the snapshot finite.
    with open(name, "rb") as r: out, truncated = snapshot(r, size)
    if not truncated: os.unlink(name)
    tx.write(json.dumps(out or "(no output)").encode("utf-8") + b"\n"); tx.flush()
