import base64, os, sys, json, tempfile, threading, traceback
null = os.devnull if hasattr(os, "devnull") else ("nul" if os.name == "nt" else "/dev/null")
rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
os.dup2(os.open(null, os.O_RDONLY), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
IMAGE_LIMIT = 5 * 1024 * 1024
IMAGE_CALL_LIMIT = 2

class Call:
    def __init__(self):
        self.images = []
        self.thread = threading.current_thread()
        self.open = True

call = None

def show_image(path):
    if call is None or not call.open or threading.current_thread() is not call.thread:
        raise ValueError("show_image attaches only during the call that started it, on that call's thread; read the picture again in a new call")
    if len(call.images) >= IMAGE_CALL_LIMIT:
        raise ValueError(f"show_image accepts at most {IMAGE_CALL_LIMIT} pictures in one python call; pass the remaining paths in a later call")
    with open(path, "rb") as source:
        data = source.read(IMAGE_LIMIT + 1)
    if len(data) > IMAGE_LIMIT:
        raise ValueError(f"show_image accepts at most {IMAGE_LIMIT} bytes per image")
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        media_type = "image/png"
    elif data.startswith(b"\xff\xd8\xff"):
        media_type = "image/jpeg"
    elif data.startswith((b"GIF87a", b"GIF89a")):
        media_type = "image/gif"
    elif len(data) >= 12 and data.startswith(b"RIFF") and data[8:12] == b"WEBP":
        media_type = "image/webp"
    else:
        raise ValueError("show_image accepts only PNG, JPEG, GIF, or WebP bytes")
    call.images.append({"media_type": media_type, "data": base64.b64encode(data).decode("ascii")})

g = {"__name__": "__main__", "show_image": show_image}
capture_dir = os.environ.pop("MEI_CAPTURE_DIR", None)
os.environ.pop("MEI_CAPTURE_CREATED", None)
os.environ.pop("MEI_STEER_PATH", None)
for line in rx:
    code = json.loads(line.decode("utf-8"))
    if not isinstance(code, str):
        raise TypeError("control frame must be a JSON string")
    current = Call()
    call = current
    f = tempfile.NamedTemporaryFile("w+b", delete=False, dir=capture_dir)
    os.dup2(f.fileno(), 1)
    os.dup2(f.fileno(), 2)
    failed = False
    try:
        exec(code, g)
    except BaseException as e:
        failed = True
        traceback.print_exception(type(e), e, e.__traceback__.tb_next)
    finally:
        current.open = False
        sys.stdout.flush()
        sys.stderr.flush()
    name = os.path.basename(f.name)
    f.close()
    frame = {"capture": name, "images": current.images, "failed": failed}
    tx.write(json.dumps(frame, separators=(",", ":")).encode("ascii") + b"\n")
    tx.flush()
