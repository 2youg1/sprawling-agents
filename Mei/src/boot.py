import os
import sys
import traceback
from _thread import get_ident

rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
with open(os.devnull, "rb") as null:
    os.dup2(null.fileno(), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
INLINE_LIMIT = int(sys.argv.pop())
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


def named_output(fd=None):
    for _ in range(16):
        name = "capture-" + os.urandom(8).hex()
        path = os.path.join(capture_dir, name)
        try:
            if fd is None:
                fd = os.open(path, CAPTURE_FLAGS, 0o600)
            else:
                os.link(f"/proc/self/fd/{fd}", path, src_dir_fd=fd)
            return name.encode("ascii"), fd
        except FileExistsError:
            continue
    raise FileExistsError(f"could not allocate a unique capture file in {capture_dir}")


def unnamed_output():
    return b"", os.open(capture_dir, os.O_RDWR | os.O_TMPFILE, 0o600)


def first_output():
    global open_output
    open_output = named_output
    if hasattr(os, "O_TMPFILE") and hasattr(os, "pread"):
        try:
            _, fd = unnamed_output()
            try:
                result = named_output(fd)
            except BaseException:
                os.close(fd)
                raise
        except OSError:
            return named_output()
        open_output = unnamed_output
        return result
    return named_output()


open_output = first_output


def capture_result(name, fd):
    if name:
        return name, b""
    size = os.fstat(fd).st_size
    if size <= INLINE_LIMIT:
        data = os.pread(fd, size, 0) if size else b""
        try:
            data.decode("utf-8")
        except UnicodeDecodeError:
            pass
        else:
            if len(data) == size:
                return b"", size.to_bytes(2, "little") + data
    name, _ = named_output(fd)
    return name, b""


def execute(code):
    name, fd = open_output()
    try:
        os.dup2(fd, 1)
        os.dup2(fd, 2)
        failed, images = evaluate(code)
        name, text = capture_result(name, fd)
    finally:
        if fd not in (1, 2):
            os.close(fd)
    tx.write(bytes((len(name), failed, len(images))) + name + text)
    for kind, data in images:
        tx.write(bytes((kind,)) + len(data).to_bytes(4, "little"))
        tx.write(data)
    tx.flush()


def evaluate(code):
    global call
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
    return failed, current.images


g = {"__name__": "__main__", "show_image": show_image}
while length := rx.read(8):
    if len(length) != 8:
        raise EOFError("truncated control frame length")
    size = int.from_bytes(length, "little")
    code = rx.read(size)
    if len(code) != size:
        raise EOFError("truncated control frame code")
    execute(code.decode("utf-8"))
