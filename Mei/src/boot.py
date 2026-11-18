import base64, os, sys, json, tempfile, traceback
null = os.devnull if hasattr(os, "devnull") else ("nul" if os.name == "nt" else "/dev/null")
rx, tx = os.fdopen(os.dup(0), "rb"), os.fdopen(os.dup(1), "wb")
os.dup2(os.open(null, os.O_RDONLY), 0)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")
IMAGE_LIMIT = 5 * 1024 * 1024
images = []

def show_image(path):
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
    images.append({"media_type": media_type, "data": base64.b64encode(data).decode("ascii")})

g = {"__name__": "__main__", "show_image": show_image}
capture_dir = os.environ.pop("MEI_CAPTURE_DIR", None)
os.environ.pop("MEI_CAPTURE_CREATED", None)
for line in rx:
    code = json.loads(line.decode("utf-8"))
    if not isinstance(code, str):
        raise TypeError("control frame must be a JSON string")
    images = []
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
    frame = {"capture": name, "images": images}
    tx.write(json.dumps(frame, separators=(",", ":")).encode("ascii") + b"\n")
    tx.flush()
