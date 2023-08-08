PAIRS = []

with open("keys.txt") as keys:
    for x in keys.readlines():
        if len(x) < 5:
            continue
        x = x.strip().split(",")
        PAIRS += [x]

from subprocess import run
import time
from os.path import exists


def getlens():
    print("reading statistics, please wait...")
    r = {}
    r["actual"] = 0
    r["all"] = 0
    if not (exists("playlists.csv") or exists("plists.txt")):
        return r

    # we can't use len here because of memory constraints,
    # thanks gc, very cool
    # so, just count the lines
    with open("playlists.csv") as t:
        for _ in t:
            r["actual"] += 1
            if r["actual"] % 1000 == 0 and r["actual"] != 0:
                out = f'actual: {r["actual"]} lines read...'
                print("\b" * len(out) + out, end="", flush=True)
        print("\b" * len(out) + out + "done", flush=True)
    with open("plists.txt") as t:
        for _ in t:
            r["all"] += 1
            if r["all"] % 1000 == 0 and r["all"] != 0:
                out = f'all {r["all"]} lines read...'
                print("\b" * len(out) + out, end="", flush=True)
        print("\b" * len(out) + out + "done", flush=True)
    return r


check = getlens()
ACTUAL_B = check["actual"]
ALL_B = check["all"]

try:
    while True:
        actual_hold = getlens()["actual"]
        death_check = time.time()
        begin = None
        for e, i in enumerate(PAIRS):
            ID = i[0]
            SECRET = i[1]
            print(f"\nkey {e + 1} out of {len(PAIRS)}")
            run(
                ["../../target/release/mio-scrape"],
                env={"RSPOTIFY_CLIENT_ID": ID, "RSPOTIFY_CLIENT_SECRET": SECRET},
            )
            if begin is None:
                begin = time.time() + (15.0 * 60.0)
        if (
            time.time() - death_check < 15.0 * 60.0
        ):  # 15 minutes, if runtime less than this then something broke
            print("runtime less than 15 minutes. something broke")
            break
        actual_new = getlens()["actual"]
        sleep_until = begin + (60.0 * 60.0 * 24.0) + (60.0 * 15.0)
        print(
            f"\nfinished epoch, gathered {actual_new - actual_hold} playlists ({actual_hold} -> {actual_new})."
            + f" sleeping until {time.strftime('%a, %d %b %Y %H:%M:%S +0000', time.gmtime(sleep_until))}\n"
        )
        spinner = ["|", "/", "-", "\\"]
        for x in range(int(sleep_until - time.time())):
            time.sleep(1)
            choice = spinner[x % len(spinner)]
            print(f"\b{choice}", flush=True, end="")
        print()
except KeyboardInterrupt:
    check = check

new = getlens()
print()
print(
    f'new playlists: {new["actual"] - ACTUAL_B} (out of {new["all"] - ALL_B}). {ACTUAL_B} -> {new["actual"]}'
)
