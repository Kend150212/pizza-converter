#!/usr/bin/env python3
"""
Pizza Converter — Two-Way Sync Script (PC Windows <-> iMac macOS)
Usage:
  python sync_pc_imac.py push   # Push latest code + AGENTS.md from PC -> iMac
  python sync_pc_imac.py pull   # Pull latest code + AGENTS.md from iMac -> PC
"""
import os, sys, tarfile, subprocess

PC_ROOT = r"c:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter"
IMAC_HOST = "imac-via-pc"
IMAC_ROOT = "/Users/kendao/agent-work/pizza-src"

SYNC_ITEMS = ["AGENTS.md", "Cargo.toml", "Cargo.lock", "crates", "src", "src-tauri"]

def filter_tar(tarinfo):
    name = tarinfo.name.replace("\\", "/")
    parts = name.split("/")
    if any(p in ("target", ".git", "node_modules", "__pycache__") for p in parts):
        return None
    if name.endswith((".exe", ".msi", ".pdb", ".dll", ".lib", ".tar.gz", ".dmg")):
        return None
    return tarinfo

def push_to_imac():
    tar_path = os.path.join(os.environ.get("TEMP", "/tmp"), "pizza_sync_push.tar.gz")
    with tarfile.open(tar_path, "w:gz") as tar:
        for item in SYNC_ITEMS:
            p = os.path.join(PC_ROOT, item)
            if os.path.exists(p):
                tar.add(p, arcname=item, filter=filter_tar)
    subprocess.run(["scp", tar_path, f"{IMAC_HOST}:{IMAC_ROOT}/sync.tar.gz"], check=True)
    remote_sh = f"""
    set -e
    cd {IMAC_ROOT}
    tar -xzf sync.tar.gz
    rm -f sync.tar.gz
    cp AGENTS.md /Users/kendao/.gemini/GEMINI.md 2>/dev/null || true
    echo "PUSH_TO_IMAC_OK"
    """
    subprocess.run(["ssh", IMAC_HOST, "bash -s"], input=remote_sh.encode("utf-8"), check=True)
    print("Successfully pushed PC -> iMac!")

def pull_from_imac():
    remote_sh = f"""
    set -e
    cd {IMAC_ROOT}
    tar --exclude='target' --exclude='.git' --exclude='__pycache__' -czf /tmp/pizza_sync_pull.tar.gz AGENTS.md Cargo.toml Cargo.lock crates src src-tauri
    echo "REMOTE_TAR_OK"
    """
    subprocess.run(["ssh", IMAC_HOST, "bash -s"], input=remote_sh.encode("utf-8"), check=True)
    tar_path = os.path.join(os.environ.get("TEMP", "/tmp"), "pizza_sync_pull.tar.gz")
    subprocess.run(["scp", f"{IMAC_HOST}:/tmp/pizza_sync_pull.tar.gz", tar_path], check=True)
    with tarfile.open(tar_path, "r:gz") as tar:
        tar.extractall(PC_ROOT)
    print("Successfully pulled iMac -> PC!")

if __name__ == "__main__":
    mode = sys.argv[1].lower() if len(sys.argv) > 1 else "push"
    if mode == "pull":
        pull_from_imac()
    elif mode == "build":
        remote_sh = """
        set -e
        export PATH="$HOME/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:$PATH"
        cd /Users/kendao/agent-work/pizza-src/src-tauri
        cargo build --release --bin pizza-converter
        pkill -9 -f pizza-converter || true
        cp -f /Users/kendao/agent-work/pizza-src/target/release/pizza-converter "/Applications/Pizza Converter.app/Contents/MacOS/pizza-converter" 2>/dev/null || true
        echo "IMAC_BUILD_AND_DEPLOY_OK"
        """
        subprocess.run(["ssh", IMAC_HOST, "bash -s"], input=remote_sh.encode("utf-8"), check=True)
    else:
        push_to_imac()
