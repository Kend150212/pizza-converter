#!/usr/bin/env python3
"""Tạo file mẫu thật cho test matrix Pie (28 files + 1 file PDF 3 trang).
Chạy 1 lần:  python3 make-samples.py [--force]
"""
import os, sys, subprocess, shutil, json

HERE = os.path.dirname(os.path.abspath(__file__))
SAMPLES = os.path.join(HERE, "samples")
FORCE = "--force" in sys.argv
os.makedirs(SAMPLES, exist_ok=True)

ok, fail = [], []

def note(name, good, msg=""):
    (ok if good else fail).append(name)
    print(("  OK  " if good else "  FAIL") + f" {name} {msg}")

def need(path):
    return FORCE or not os.path.exists(path)

def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=120, **kw)

# ---------- 1. IMAGES ----------
def make_images():
    from PIL import Image
    img = Image.new("RGB", (256, 256))
    px = img.load()
    for y in range(256):
        for x in range(256):
            px[x, y] = (x, y, (x + y) // 2)
    jobs = [("sample.png", "PNG"), ("sample.jpg", "JPEG"), ("sample.bmp", "BMP"),
            ("sample.gif", "GIF"), ("sample.tif", "TIFF"), ("sample.ico", "ICO"),
            ("sample.webp", "WEBP")]
    for name, fmt in jobs:
        p = os.path.join(SAMPLES, name)
        if not need(p):
            note(name, True, "(có sẵn)"); continue
        try:
            img.save(p, fmt); note(name, True)
        except Exception as e:
            note(name, False, str(e)[:80])
    # AVIF via ffmpeg (PIL 10.2 không có AVIF)
    p = os.path.join(SAMPLES, "sample.avif")
    if need(p):
        r = run(["ffmpeg", "-y", "-v", "error", "-i", os.path.join(SAMPLES, "sample.png"),
                 "-c:v", "libaom-av1", "-crf", "30", p])
        note("sample.avif", r.returncode == 0 and os.path.exists(p), r.stderr.strip()[:80])
    else:
        note("sample.avif", True, "(có sẵn)")
    # HEIC via pillow-heif
    p = os.path.join(SAMPLES, "sample.heic")
    if need(p):
        try:
            import pillow_heif
            heif = pillow_heif.from_pillow(img)
            heif.save(p)
            note("sample.heic", True)
        except Exception as e:
            note("sample.heic", False, str(e)[:80])
    else:
        note("sample.heic", True, "(có sẵn)")

# ---------- 2. DOCUMENTS ----------
LOREM_VI = ("Pizza Converter. Chạm một lát, xong ngay.\n"
            "Pick a slice, convert in a flick.\n"
            "Dòng tiếng Việt có dấu: á à ả ã ạ ă â đ ê ô ơ ư.\n" * 6)

def make_documents():
    # TXT
    p = os.path.join(SAMPLES, "sample.txt")
    if need(p):
        open(p, "w", encoding="utf-8").write(LOREM_VI)
    note("sample.txt", os.path.exists(p))
    # RTF (viết tay, tối giản hợp lệ)
    p = os.path.join(SAMPLES, "sample.rtf")
    if need(p):
        body = LOREM_VI.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")
        body = body.replace("\n", "\\par\n")
        open(p, "w", encoding="utf-8").write(
            "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fswiss Arial;}}\\f0\\fs24\n" + body + "}")
    note("sample.rtf", os.path.exists(p))
    # DOCX via python-docx
    p = os.path.join(SAMPLES, "sample.docx")
    if need(p):
        try:
            from docx import Document
            d = Document()
            d.add_heading("Pizza Converter (mẫu test)", 1)
            d.add_paragraph(LOREM_VI)
            t = d.add_table(rows=3, cols=2)
            for i, row in enumerate(t.rows):
                row.cells[0].text, row.cells[1].text = f"Hàng {i+1}A", f"Hàng {i+1}B"
            d.save(p); note("sample.docx", True)
        except Exception as e:
            note("sample.docx", False, str(e)[:80])
    else:
        note("sample.docx", True, "(có sẵn)")
    # PDF / DOC / ODT via LibreOffice từ DOCX
    docx = os.path.join(SAMPLES, "sample.docx")
    for name, fmt in [("sample.pdf", "pdf"), ("sample.doc", "doc"), ("sample.odt", "odt")]:
        p = os.path.join(SAMPLES, name)
        if not need(p):
            note(name, True, "(có sẵn)"); continue
        if not os.path.exists(docx):
            note(name, False, "thiếu sample.docx"); continue
        r = run(["soffice", "--headless", "--convert-to", fmt, "--outdir", SAMPLES, docx])
        got = os.path.exists(p)
        note(name, got, "" if got else r.stderr.strip()[-80:])
    # PDF 3 trang cho test Split/Merge
    p3 = os.path.join(SAMPLES, "sample-3pages.pdf")
    if need(p3):
        try:
            from docx import Document
            from docx.enum.text import WD_BREAK
            d = Document()
            for i in range(1, 4):
                d.add_heading(f"Trang {i}", 1)
                d.add_paragraph(f"Nội dung trang {i}.\n" * 20)
                if i < 3:
                    d.add_paragraph().add_run().add_break(WD_BREAK.PAGE)
            tmp = os.path.join(SAMPLES, "_3pages.docx")
            d.save(tmp)
            r = run(["soffice", "--headless", "--convert-to", "pdf", "--outdir", SAMPLES, tmp])
            gen = os.path.join(SAMPLES, "_3pages.pdf")
            if os.path.exists(gen):
                os.rename(gen, p3)
            os.remove(tmp)
            note("sample-3pages.pdf", os.path.exists(p3))
        except Exception as e:
            note("sample-3pages.pdf", False, str(e)[:80])
    else:
        note("sample-3pages.pdf", True, "(có sẵn)")

# ---------- 3. DATA ----------
def make_data():
    rows = [["id", "name", "price"], ["1", "Bánh mì", "15000"], ["2", "Cà phê", "25000"], ["3", "Phở", "45000"]]
    p = os.path.join(SAMPLES, "sample.csv")
    if need(p):
        open(p, "w", encoding="utf-8").write("\n".join(",".join(r) for r in rows))
    note("sample.csv", os.path.exists(p))
    p = os.path.join(SAMPLES, "sample.json")
    if need(p):
        json.dump([dict(zip(rows[0], r)) for r in rows[1:]], open(p, "w", encoding="utf-8"),
                  ensure_ascii=False, indent=2)
    note("sample.json", os.path.exists(p))
    p = os.path.join(SAMPLES, "sample.xml")
    if need(p):
        open(p, "w", encoding="utf-8").write(
            '<?xml version="1.0" encoding="UTF-8"?>\n<items>\n' +
            "".join(f'  <item id="{r[0]}"><name>{r[1]}</name><price>{r[2]}</price></item>\n' for r in rows[1:]) +
            "</items>\n")
    note("sample.xml", os.path.exists(p))
    p = os.path.join(SAMPLES, "sample.yaml")
    if need(p):
        open(p, "w", encoding="utf-8").write(
            "items:\n" + "".join(f"  - id: {r[0]}\n    name: {r[1]}\n    price: {r[2]}\n" for r in rows[1:]))
    note("sample.yaml", os.path.exists(p))
    p = os.path.join(SAMPLES, "sample.xlsx")
    if need(p):
        try:
            import openpyxl
            wb = openpyxl.Workbook(); ws = wb.active
            for r in rows:
                ws.append(r)
            wb.save(p); note("sample.xlsx", True)
        except Exception as e:
            note("sample.xlsx", False, str(e)[:80])
    else:
        note("sample.xlsx", True, "(có sẵn)")

# ---------- 4. AUDIO / VIDEO ----------
def make_audio():
    specs = [("sample.wav", ["-c:a", "pcm_s16le"]),
             ("sample.mp3", ["-c:a", "libmp3lame", "-b:a", "128k"]),
             ("sample.flac", ["-c:a", "flac"]),
             ("sample.ogg", ["-c:a", "libvorbis"]),
             ("sample.m4a", ["-c:a", "aac", "-b:a", "128k"])]
    for name, acodec in specs:
        p = os.path.join(SAMPLES, name)
        if not need(p):
            note(name, True, "(có sẵn)"); continue
        r = run(["ffmpeg", "-y", "-v", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=2",
                 "-ar", "44100"] + acodec + [p])
        note(name, r.returncode == 0 and os.path.exists(p), r.stderr.strip()[:80])

def make_video():
    venc = "libx264" if b"libx264" in subprocess.run(
        ["ffmpeg", "-hide_banner", "-encoders"], capture_output=True).stdout else "mpeg4"
    specs = [("sample.mp4", ["-c:v", venc, "-pix_fmt", "yuv420p"]),
             ("sample.mkv", ["-c:v", venc, "-pix_fmt", "yuv420p"]),
             ("sample.webm", ["-c:v", "libvpx-vp9", "-b:v", "500k", "-c:a", "libopus"])]
    for name, vcodec in specs:
        p = os.path.join(SAMPLES, name)
        if not need(p):
            note(name, True, "(có sẵn)"); continue
        r = run(["ffmpeg", "-y", "-v", "error", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=30:duration=2",
                 "-f", "lavfi", "-i", "sine=frequency=440:duration=2"] + vcodec +
                ["-c:a", "aac", "-shortest", p])
        note(name, r.returncode == 0 and os.path.exists(p), r.stderr.strip()[:80])

if __name__ == "__main__":
    print("== Images =="); make_images()
    print("== Documents =="); make_documents()
    print("== Data =="); make_data()
    print("== Audio =="); make_audio()
    print("== Video =="); make_video()
    print(f"\nXONG: {len(ok)} OK, {len(fail)} FAIL")
    if fail:
        print("FAIL:", ", ".join(fail)); sys.exit(1)
