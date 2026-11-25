import os
import sys
import traceback
from _thread import get_ident

rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
with open(os.devnull, "rb") as null:
    os.dup2(null.fileno(), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
IMAGE_LIMIT = 5 * 1024 * 1024
IMAGE_CALL_LIMIT = 2
CAPTURE_FLAGS = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_BINARY", 0)
capture_dir = os.environ.pop("MEI_CAPTURE_DIR")
os.environ.pop("MEI_CAPTURE_CREATED", None)
os.environ.pop("MEI_STEER_PATH", None)


class Call:
    def __init__(self):
        self.images = []
        self.thread = get_ident()


call = None


def show_image(path):
    if call is None or get_ident() != call.thread:
        raise ValueError("show_image attaches only during the call that started it, on that call's thread; read the picture again in a new call")
    if len(call.images) >= IMAGE_CALL_LIMIT:
        raise ValueError(f"show_image accepts at most {IMAGE_CALL_LIMIT} pictures in one python call; pass the remaining paths in a later call")
    with open(path, "rb") as source:
        data = source.read(IMAGE_LIMIT + 1)
    if len(data) > IMAGE_LIMIT:
        raise ValueError(f"show_image accepts at most {IMAGE_LIMIT} bytes per image")
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        kind = 1
    elif data.startswith(b"\xff\xd8\xff"):
        kind = 2
    elif data.startswith((b"GIF87a", b"GIF89a")):
        kind = 3
    elif len(data) >= 12 and data.startswith(b"RIFF") and data[8:12] == b"WEBP":
        kind = 4
    else:
        raise ValueError("show_image accepts only PNG, JPEG, GIF, or WebP bytes")
    call.images.append((kind, data))


def redirect_output():
    for _ in range(16):
        name = "capture-" + os.urandom(8).hex()
        try:
            fd = os.open(os.path.join(capture_dir, name), CAPTURE_FLAGS, 0o600)
        except FileExistsError:
            continue
        break
    else:
        raise FileExistsError(f"could not allocate a unique capture file in {capture_dir}")
    try:
        os.dup2(fd, 1)
        os.dup2(fd, 2)
    finally:
        if fd not in (1, 2):
            os.close(fd)
    return name.encode("ascii")


def execute(code):
    global call
    name = redirect_output()
    current = Call()
    call = current
    failed = False
    try:
        exec(code, g)
    except BaseException as e:
        failed = True
        traceback.print_exception(type(e), e, e.__traceback__.tb_next)
    finally:
        call = None
        sys.stdout.flush()
        sys.stderr.flush()
    tx.write(bytes((len(name), failed, len(current.images))) + name)
    for kind, data in current.images:
        tx.write(bytes((kind,)) + len(data).to_bytes(4, "little"))
        tx.write(data)
    tx.flush()


g = {"__name__": "__main__", "show_image": show_image}
while length := rx.read(8):
    if len(length) != 8:
        raise EOFError("truncated control frame length")
    size = int.from_bytes(length, "little")
    code = rx.read(size)
    if len(code) != size:
        raise EOFError("truncated control frame code")
    execute(code.decode("utf-8"))
