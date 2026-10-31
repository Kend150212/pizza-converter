#!/usr/bin/env python3
"""Chạy test matrix Pie.
  python3 run-matrix.py --selfcheck
      Tự kiểm: mỗi file mẫu trong samples/ phải verify đúng định dạng của nó
      (chứng minh file mẫu hợp lệ + verify.py hoạt động).
  python3 run-matrix.py --bin pizza-convert --out results/build-001
      Chạy toàn bộ 184 cases qua lệnh convert của Dev:
      pizza-convert <input> <output>   (Dev cung cấp CLI ẩn gọi đúng engine thật)
      Kết quả -> results/build-001.json ; case lỗi in cuối để báo Dev sửa ngay.
"""
import os, sys, json, subprocess, argparse, datetime
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from verify import verify_file

HERE = os.path.dirname(os.path.abspath(__file__))
SAMPLES = os.path.join(HERE, "samples")

IMAGES = ["PNG", "JPG", "WEBP", "AVIF", "BMP", "ICO", "GIF", "TIFF", "HEIC"]
DOCS = ["TXT", "DOCX", "PDF", "DOC", "ODT", "RTF"]
DATA = ["XLSX", "CSV", "JSON", "XML", "YAML"]
AUDIO = ["MP3", "WAV", "FLAC", "OGG", "M4A"]
VIDEO = ["MP4", "MKV", "WEBM"]

def ext(f):
    return {"JPG": "jpg", "TIFF": "tif"}.get(f, f.lower())

def cases():
    cs = []
    for i in IMAGES:
        for o in IMAGES:
            cs.append((i, o, f"sample.{ext(i)}", f"images/{ext(i)}-to-{ext(o)}.{ext(o)}"))
    for i in DOCS:
        for o in DOCS:
            cs.append((i, o, f"sample.{ext(i)}", f"docs/{ext(i)}-to-{ext(o)}.{ext(o)}"))
    for i in DATA:
        for o in DATA:
            cs.append((i, o, f"sample.{ext(i)}", f"data/{ext(i)}-to-{ext(o)}.{ext(o)}"))
    cs += [("PDF", "DOCX", "sample.pdf", "pdftools/pdf-to-docx.docx"),
           ("PDF", "TXT", "sample.pdf", "pdftools/pdf-to-txt.txt"),
           ("PDF", "PNG", "sample.pdf", "pdftools/pdf-to-png.png")]
    cs.append(("PDF_OP_COMPRESS", "PDF", "sample.pdf", "pdftools/compress.pdf"))
    cs.append(("PDF_OP_SPLIT", "PDF", "sample-3pages.pdf", "pdftools/split/"))
    cs.append(("PDF_OP_MERGE", "PDF", "sample.pdf+sample-3pages.pdf", "pdftools/merged.pdf"))
    for i in AUDIO:
        for o in AUDIO:
            cs.append((i, o, f"sample.{ext(i)}", f"audio/{ext(i)}-to-{ext(o)}.{ext(o)}"))
    for i in VIDEO:
        for o in VIDEO:
            cs.append((i, o, f"sample.{ext(i)}", f"video/{ext(i)}-to-{ext(o)}.{ext(o)}"))
    cs += [("MP4", "MP3", "sample.mp4", "cross/mp4-to-mp3.mp3"),
           ("MP4", "WAV", "sample.mp4", "cross/mp4-to-wav.wav")]
    # Office -> PDF (vượt app gốc, lệnh anh Ken 29/10 11:40)
    for src in ["sample.docx", "sample.xlsx", "sample.pptx", "sample.odt", "sample.rtf", "sample.txt"]:
        tag = src.replace("sample.", "").replace(".", "-")
        cs.append(("OFFICE", "PDF", src, f"officepdf/{tag}-to-pdf.pdf"))
    return cs

def selfcheck():
    print(f"Tự kiểm {len(os.listdir(SAMPLES))} file mẫu...")
    bad = []
    for name in sorted(os.listdir(SAMPLES)):
        fmt = name.rsplit(".", 1)[-1].upper()
        fmt = {"JPG": "JPG", "TIF": "TIFF"}.get(fmt, fmt)
        ok, msg = verify_file(os.path.join(SAMPLES, name), fmt)
        print(("  PASS " if ok else "  FAIL ") + f"{name}: {msg}")
        if not ok:
            bad.append(name)
    print(f"\nSELFCHECK: {len(os.listdir(SAMPLES)) - len(bad)}/{len(os.listdir(SAMPLES))} pass")
    return not bad

def is_v2(fi, fo):
    # v2: xuất HEIC (Điều phối chốt 29/10, quyền anh Ken) — loại khỏi gate v1.
    return fo == "HEIC"

def full_run(bin_cmd, outdir, include_v2=False):
    cs = cases()
    if not include_v2:
        cs = [c for c in cs if not is_v2(c[0], c[1])]
    print(f"Chạy {len(cs)} cases qua: {bin_cmd} <in> <out>" + ("" if not include_v2 else " (+v2)"))
    os.makedirs(outdir, exist_ok=True)
    results = []
    for n, (fi, fo, src, rel) in enumerate(cs, 1):
        dst = os.path.join(outdir, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        conv_ok, conv_msg = True, ""
        if fi == "PDF_OP_COMPRESS":
            # pizza-convert --pro <in.pdf> <out.pdf> --compress
            try:
                r = subprocess.run([bin_cmd, "--pro", os.path.join(SAMPLES, src), dst, "--compress"],
                                   capture_output=True, text=True, timeout=300)
                conv_ok = r.returncode == 0
                conv_msg = r.stderr.strip()[-120:] if not conv_ok else ""
            except Exception as e:
                conv_ok, conv_msg = False, str(e)[:120]
        elif fi == "PDF_OP_SPLIT":
            # pizza-convert --pro <3pages.pdf> --split <dir> → 3 file, mỗi file 1 trang
            try:
                splitdir = dst
                os.makedirs(splitdir, exist_ok=True)
                r = subprocess.run([bin_cmd, "--pro", os.path.join(SAMPLES, src), "--split", splitdir],
                                   capture_output=True, text=True, timeout=300)
                conv_ok = r.returncode == 0
                conv_msg = r.stderr.strip()[-120:] if not conv_ok else ""
                if conv_ok:
                    parts = sorted(f for f in os.listdir(splitdir) if f.endswith(".pdf"))
                    conv_ok = len(parts) == 3
                    conv_msg = "" if conv_ok else f"split ra {len(parts)} file, cần 3"
                    dst = os.path.join(splitdir, parts[0]) if parts else dst
            except Exception as e:
                conv_ok, conv_msg = False, str(e)[:120]
        elif fi == "PDF_OP_MERGE":
            # pizza-convert --pro <a.pdf,b.pdf> <merged.pdf> --merge
            try:
                inputs = ",".join(os.path.join(SAMPLES, s) for s in src.split("+"))
                r = subprocess.run([bin_cmd, "--pro", inputs, dst, "--merge"],
                                   capture_output=True, text=True, timeout=300)
                conv_ok = r.returncode == 0
                conv_msg = r.stderr.strip()[-120:] if not conv_ok else ""
            except Exception as e:
                conv_ok, conv_msg = False, str(e)[:120]
        elif fi.startswith("PDF_OP_"):
            conv_ok, conv_msg = False, "op đặc biệt: chạy tay theo test-matrix.md §4"
        else:
            try:
                # Truyền --pro để test cả format Pro (AVIF/TIFF...) — kiểm tra
                # capability; khóa bản quyền Pro test riêng ở tầng UI.
                r = subprocess.run([bin_cmd, "--pro", os.path.join(SAMPLES, src), dst],
                                   capture_output=True, text=True, timeout=300)
                conv_ok = r.returncode == 0
                conv_msg = r.stderr.strip()[-120:] if not conv_ok else ""
            except Exception as e:
                conv_ok, conv_msg = False, str(e)[:120]
        vok, vmsg = (False, "chưa convert") if not conv_ok else verify_file(dst, fo)
        passed = conv_ok and vok
        results.append({"case": f"{fi}->{fo}", "src": src, "out": rel,
                        "pass": passed, "convert": conv_msg, "verify": vmsg})
        print(f"  [{'PASS' if passed else 'FAIL'}] {n}/{len(cs)} {fi}->{fo} {vmsg}")
    fails = [r for r in results if not r["pass"]]
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M")
    jf = os.path.join(HERE, "results", os.path.basename(outdir.rstrip("/")) + ".json")
    os.makedirs(os.path.dirname(jf), exist_ok=True)
    json.dump({"build": os.path.basename(outdir), "time": stamp,
               "total": len(results), "passed": len(results) - len(fails),
               "results": results}, open(jf, "w"), ensure_ascii=False, indent=1)
    print(f"\nKẾT QUẢ: {len(results) - len(fails)}/{len(results)} pass → {jf}")
    if fails:
        print("CASE LỖI (báo Dev sửa ngay):")
        for r in fails:
            print(f"  - {r['case']}: convert={r['convert']} verify={r['verify']}")
        return False
    return True

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--selfcheck", action="store_true")
    ap.add_argument("--bin", default="")
    ap.add_argument("--out", default="results/manual")
    ap.add_argument("--include-v2", action="store_true",
                    help="chạy cả case v2 (xuất HEIC); mặc định bỏ qua khỏi gate v1")
    a = ap.parse_args()
    if a.selfcheck:
        sys.exit(0 if selfcheck() else 1)
    if not a.bin:
        print("Thiếu --bin (lệnh convert của Dev). Dùng --selfcheck để tự kiểm file mẫu.")
        sys.exit(2)
    sys.exit(0 if full_run(a.bin, a.out, a.include_v2) else 1)
