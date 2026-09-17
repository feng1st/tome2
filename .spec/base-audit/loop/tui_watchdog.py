#!/usr/bin/env python3
"""看门狗：盯着本会话（这个 Konsole 窗口），一旦停止就模拟键盘输入强制继续。

- 活动监测：opencode 会话数据库 `opencode.db-wal` 的 mtime
  （本会话每条消息/每次工具调用都会写它）
- 停止判定：连续 IDLE 秒没有任何写入，且严格验收不是 ACCEPTED
- 强制继续：Konsole D-Bus `org.kde.konsole.Session.sendText` 往本窗口
  注入一条继续指令 + 回车，等价于有人在本窗口敲字
- 结束条件：`VERDICT: ACCEPTED` / STOP 文件 / MAX_NUDGES 用尽
- 不使用 headless：不启动新的 opencode 进程，只驱动当前会话

用法：
    python3 .spec/base-audit/loop/tui_watchdog.py
    nohup python3 .spec/base-audit/loop/tui_watchdog.py &   # 后台

环境变量：
    IDLE=180        停止多少秒后注入（默认 180）
    POLL=15         活动轮询间隔秒（默认 15）
    CHECK=600       每隔多少秒跑一次严格验收（默认 600）
    COOLDOWN=240    一次注入后至少等多少秒再注入下一次
    MAX_NUDGES=400  最多注入次数
    DRY_RUN=1       只记录不注入（用于测试）
    PROMPT=...      注入的继续指令
    RUNDIR=...      仓库根目录
停止：`touch .spec/base-audit/loop/tui_watchdog.stop`
"""
import os
import re
import shutil
import subprocess
import sys
import time

RUNDIR = os.environ.get(
    "RUNDIR", os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
)
LOOP = os.path.join(RUNDIR, ".spec", "base-audit", "loop")
LOG = os.path.join(LOOP, "tui_watchdog.log")
STOP = os.path.join(LOOP, "tui_watchdog.stop")
DB_WAL = os.environ.get(
    "DB_WAL", os.path.expanduser("~/.local/share/opencode/opencode.db-wal")
)

IDLE = int(os.environ.get("IDLE", "180"))
POLL = int(os.environ.get("POLL", "15"))
CHECK = int(os.environ.get("CHECK", "600"))
COOLDOWN = int(os.environ.get("COOLDOWN", "240"))
MAX_NUDGES = int(os.environ.get("MAX_NUDGES", "400"))
DRY_RUN = os.environ.get("DRY_RUN", "0") == "1"
PROMPT = os.environ.get(
    "PROMPT",
    "继续 audit2：读 .spec/audit2/README.md 与 .spec/audit2/checklist/。"
    "对每个原版条目，在 bevy/src 找到真实实现，逐行比对逻辑（坑/边界/顺序），"
    "需要时补测试，然后在该行写 `impl:bevy/src/<file>.rs:<line> test:<module::test>`"
    "（logic: 只作为给人看的结论，不作证据）。HUD 与 lib/mods/theme 豁免。"
    "不要问、不要停、不要中途汇报，直到 `python3 .spec/audit2/audit.py` 输出 "
    "VERDICT: ACCEPTED。",
)

DBUS = shutil.which("dbus-send") or "/home/linuxbrew/.linuxbrew/bin/dbus-send"
SERVICE = os.environ.get("KONSOLE_DBUS_SERVICE", "")
SESSION = os.environ.get("KONSOLE_DBUS_SESSION", "")


def log(msg):
    line = "%s %s" % (time.strftime("%F %T"), msg)
    with open(LOG, "a") as f:
        f.write(line + "\n")
    print(line, flush=True)


def activity_mtime():
    try:
        return os.stat(DB_WAL).st_mtime
    except OSError:
        try:
            return os.stat(DB_WAL[:-4]).st_mtime
        except OSError:
            return 0.0


def send_text(text):
    """Simulate typing `text` into this Konsole session via D-Bus."""
    if DRY_RUN:
        log("[DRY_RUN] sendText %r" % text)
        return True
    cmd = [
        DBUS,
        "--session",
        "--print-reply",  # surface method errors (without it rc=0 lies)
        "--type=method_call",
        "--dest=" + SERVICE,
        SESSION,
        "org.kde.konsole.Session.sendText",
        "string:" + text,
    ]
    r = subprocess.run(cmd, capture_output=True, text=True)
    out = r.stdout + r.stderr
    if r.returncode != 0 or "Error" in out:
        log("sendText 失败 rc=%d %s" % (r.returncode, out.strip()[:200]))
        return False
    return True


def nudge():
    if not SERVICE or not SESSION:
        log("缺少 KONSOLE_DBUS_SERVICE/SESSION，无法注入")
        return False
    if not send_text(PROMPT):
        return False
    time.sleep(0.3)
    if not send_text("\r"):  # Enter
        return False
    return True


def check_accept():
    """Run the configured acceptance checker; return (verdict, open_count).
    AUDIT_CMD points at audit2/audit.py by default."""
    cmd = os.environ.get(
        "AUDIT_CMD", os.path.join(RUNDIR, ".spec", "audit2", "audit.py")
    )
    try:
        r = subprocess.run(
            [sys.executable] + cmd.split(),
            capture_output=True,
            text=True,
            cwd=RUNDIR,
            timeout=3600,
        )
    except subprocess.TimeoutExpired:
        log("audit 超时")
        return ("TIMEOUT", None)
    out = r.stdout
    verdict = None
    m = re.search(r"^VERDICT:\s*(\S+)", out, re.M)
    if m:
        verdict = m.group(1)
    open_n = None
    m = re.search(r"violations:\s*(\d+)", out)
    if m:
        open_n = int(m.group(1))
    return (verdict, open_n)


def main():
    if not SERVICE or not SESSION:
        log("警告：环境里没有 KONSOLE_DBUS_SERVICE/KONSOLE_DBUS_SESSION，注入会失败")
    log("看门狗启动：idle=%ds poll=%ds check=%ds cooldown=%ds dry=%s" % (IDLE, POLL, CHECK, COOLDOWN, DRY_RUN))
    log("窗口：service=%s session=%s" % (SERVICE, SESSION))

    verdict, open_n = check_accept()
    log("首次验收：verdict=%s open=%s" % (verdict, open_n))
    if verdict == "ACCEPTED":
        log("已经 ACCEPTED，退出")
        return

    last_activity = activity_mtime()
    last_check = time.time()
    last_nudge = 0.0
    nudges = 0

    while True:
        if os.path.exists(STOP):
            log("发现 STOP 文件，退出")
            return
        if nudges >= MAX_NUDGES:
            log("达到 MAX_NUDGES=%d，退出" % MAX_NUDGES)
            return

        time.sleep(POLL)

        now = time.time()
        act = activity_mtime()
        if act != last_activity:
            last_activity = act

        if now - last_check >= CHECK:
            verdict, open_n = check_accept()
            last_check = now
            log("周期验收：verdict=%s open=%s" % (verdict, open_n))
            if verdict == "ACCEPTED":
                log("完成：ACCEPTED")
                return

        idle_for = now - last_activity
        cooled = now - last_nudge >= COOLDOWN
        if idle_for >= IDLE and cooled:
            log(
                "检测到停止 %ds（open=%s），注入第 %d 次"
                % (int(idle_for), open_n, nudges + 1)
            )
            if nudge():
                nudges += 1
                last_nudge = time.time()
                # 给注入留出反应时间，避免连续灌
                last_activity = activity_mtime()


if __name__ == "__main__":
    main()
