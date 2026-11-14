import os, sys, json, signal, tempfile, traceback, codecs
rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
os.dup2(os.open(os.devnull, os.O_RDONLY), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
g = {"__name__": "__main__"}
def boom(*_): raise TimeoutError("timeout after 120s")
def snapshot(f):
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    head, tail, count = "", "", 0
    while True:
        chunk = f.read(65536)
        if not chunk: break
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
    signal.alarm(120)
    try: exec(code, g)
    except BaseException as e: traceback.print_exception(type(e), e, e.__traceback__.tb_next)
    finally: signal.alarm(0)
    sys.stdout.flush(); sys.stderr.flush()
    f.seek(0); out, truncated = snapshot(f); f.close()
    if not truncated: os.unlink(f.name)
    tx.write(json.dumps(out or "(no output)").encode("utf-8") + b"\n"); tx.flush()
