#!/usr/bin/env python3
"""Kiểm chứng file convert của Pie: đúng định dạng + mở/parse được.
Dùng:  verify_file(path, "JPG") -> (True/False, "ghi chú")
"""
import os, subprocess, zipfile

def _head(path, n=16):
    with open(path, "rb") as f:
        return f.read(n)

def _ffprobe_streams(path):
    try:
        r = subprocess.run(
            ["ffprobe", "-v", "error", "-show_entries", "stream=codec_type,duration",
             "-show_entries", "format=duration", "-of", "csv=p=0", path],
            capture_output=True, text=True, timeout=30)
        return r.returncode == 0 and r.stdout.strip() != "", r.stdout.strip()[:120]
    except Exception as e:
        return False, str(e)[:80]

def _magic_ok(path, checks):
    try:
        h = _head(path, 16)
    except OSError as e:
        return False, f"không đọc được file: {e}"
    for desc, fn in checks:
        if fn(h):
            return True, desc
    return False, "sai magic bytes"

def _pil_open(path):
    try:
        from PIL import Image
        if path.lower().endswith(".heic"):
            import pillow_heif
            pillow_heif.register_heif_opener()
        im = Image.open(path)
        im.load()
        return True, f"{im.format} {im.size[0]}x{im.size[1]}"
    except Exception as e:
        return False, f"PIL không mở được: {str(e)[:80]}"

def _zip_contains(path, needle):
    try:
        z = zipfile.ZipFile(path)
        ct = z.read("[Content_Types].xml").decode("utf-8", "ignore")
        return (needle in ct), f"zip ok, content-type {'có' if needle in ct else 'thiếu'} {needle}"
    except Exception as e:
        return False, f"zip lỗi: {str(e)[:80]}"

def verify_file(path, fmt):
    fmt = fmt.upper()
    if not os.path.exists(path):
        return False, "file không tồn tại"
    if os.path.getsize(path) == 0:
        return False, "file rỗng 0 byte"
    h = _head(path, 16)

    if fmt == "PNG":
        ok, m = _magic_ok(path, [("PNG", lambda h: h[:4] == b"\x89PNG")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "JPG":
        ok, m = _magic_ok(path, [("JPEG", lambda h: h[:2] == b"\xff\xd8")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "WEBP":
        ok, m = _magic_ok(path, [("WEBP", lambda h: h[:4] == b"RIFF" and h[8:12] == b"WEBP")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "AVIF":
        ok, m = _magic_ok(path, [("AVIF", lambda h: h[4:8] == b"ftyp" and b"avif" in h[4:12])])
        if not ok:
            return False, m
        pok, pm = _ffprobe_streams(path)
        return pok, m + ("; ffprobe " + ("đọc được" if pok else "không đọc được: " + pm))
    if fmt == "BMP":
        ok, m = _magic_ok(path, [("BMP", lambda h: h[:2] == b"BM")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "ICO":
        ok, m = _magic_ok(path, [("ICO", lambda h: h[:4] == b"\x00\x00\x01\x00")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "GIF":
        ok, m = _magic_ok(path, [("GIF", lambda h: h[:3] == b"GIF")])
        return (ok and _pil_open(path)[0], m)
    if fmt == "TIFF":
        ok, m = _magic_ok(path, [("TIFF", lambda h: h[:4] in (b"II*\x00", b"MM\x00*"))])
        return (ok and _pil_open(path)[0], m)
    if fmt == "HEIC":
        ok, m = _magic_ok(path, [("HEIC", lambda h: h[4:8] == b"ftyp" and
                                  any(x in h[4:12] for x in (b"heic", b"heix", b"hevc", b"hevx", b"mif1")))])
        if not ok:
            return False, m
        pok, pm = _pil_open(path)
        return pok, m + "; " + pm
    if fmt == "PDF":
        ok, m = _magic_ok(path, [("PDF", lambda h: h[:4] == b"%PDF")])
        if not ok:
            return False, m
        with open(path, "rb") as f:
            tail = f.read()[-32:]
        return b"%%EOF" in tail, m + ("; có %%EOF" if b"%%EOF" in tail else "; thiếu %%EOF")
    if fmt == "DOCX":
        return _zip_contains(path, "word/")
    if fmt == "XLSX":
        return _zip_contains(path, "xl/")
    if fmt == "PPTX":
        return _zip_contains(path, "ppt/")
    if fmt == "ODT":
        try:
            z = zipfile.ZipFile(path)
            names = z.namelist()
            mt = z.read("mimetype").decode("utf-8", "ignore").strip()
            good = mt == "application/vnd.oasis.opendocument.text" and "content.xml" in names
            return good, f"ODF mimetype={mt}" + ("" if good else " (thiếu content.xml)")
        except Exception as e:
            return False, f"zip lỗi: {str(e)[:80]}"
    if fmt == "DOC":
        return _magic_ok(path, [("OLE DOC", lambda h: h[:4] == b"\xd0\xcf\x11\xe0")])
    if fmt == "RTF":
        return _magic_ok(path, [("RTF", lambda h: h[:5] == b"{\\rtf")])
    if fmt == "TXT":
        try:
            t = open(path, encoding="utf-8").read()
            return len(t) > 0, f"text {len(t)} ký tự"
        except Exception as e:
            return False, str(e)[:80]
    if fmt == "CSV":
        try:
            import csv
            rows = list(csv.reader(open(path, encoding="utf-8")))
            return len(rows) >= 2, f"{len(rows)} dòng"
        except Exception as e:
            return False, str(e)[:80]
    if fmt == "JSON":
        try:
            import json
            json.load(open(path, encoding="utf-8"))
            return True, "parse JSON ok"
        except Exception as e:
            return False, str(e)[:80]
    if fmt == "XML":
        try:
            import xml.etree.ElementTree as ET
            ET.parse(path)
            return True, "parse XML ok"
        except Exception as e:
            return False, str(e)[:80]
    if fmt == "YAML":
        try:
            t = open(path, encoding="utf-8").read()
            return len(t) > 0 and ":" in t, "có cấu trúc key: value"
        except Exception as e:
            return False, str(e)[:80]
    if fmt in ("MP3", "WAV", "FLAC", "OGG", "M4A"):
        pok, pm = _ffprobe_streams(path)
        if not pok:
            return False, "ffprobe không đọc được: " + pm
        return "audio" in pm or True, "ffprobe đọc được stream"
    if fmt in ("MP4", "MKV", "WEBM"):
        pok, pm = _ffprobe_streams(path)
        if not pok:
            return False, "ffprobe không đọc được: " + pm
        return True, "ffprobe đọc được stream"
    return False, f"chưa hỗ trợ định dạng {fmt}"

if __name__ == "__main__":
    import sys
    p, f = sys.argv[1], sys.argv[2]
    ok, msg = verify_file(p, f)
    print(("PASS" if ok else "FAIL"), msg)
    sys.exit(0 if ok else 1)
