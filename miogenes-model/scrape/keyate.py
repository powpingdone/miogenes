PAIRS = [
#   adding keys in form of:
#   ['IDIDIDIDIDIDIDIDIDIDIDIDIDIDIDID', 'SECRETSECRETSECRETSECRETSECRETSE']
]

from subprocess import run
import time

def getlens():
    r = {}
    with open('playlists.csv') as t:
        r['actual'] = len(t.readlines())
    with open('plists.txt') as t:
        r['all'] = len(t.readlines())
    return r

check = getlens()
ACTUAL_B = check['actual']
ALL_B = check['all']

try:
    while True:
        actual_hold = getlens()['actual']
        death_check = time.time()
        begin = None
        for (e, i) in enumerate(PAIRS):
            ID = i[0]
            SECRET = i[1]
            print(f'key {e + 1} out of {len(PAIRS)}')
            run(['./target/release/scrape'], env={"RSPOTIFY_CLIENT_ID": ID, "RSPOTIFY_CLIENT_SECRET": SECRET})
            if begin is None:
                begin = time.time() + (15.0 * 60.0)
        if time.time() - death_check < 15.0 * 60.0: # 15 minutes, if runtime less than this then something broke
            print("runtime less than 15 minutes. something broke")
            break
        actual_new = getlens()['actual']
        sleep_until = begin + (60.0 * 60.0 * 24.0)
        print(f"\nfinished epoch, gathered {actual_new - actual_hold} playlists (actual_hold -> actual_new). sleeping until {time.strftime('%a, %d %b %Y %H:%M:%S +0000', time.gmtime(sleep_until))}\n")
        time.sleep(sleep_until)
except KeyboardInterrupt:
    check = check

new = getlens()
print()
print(f'new playlists: {new["actual"] - ACTUAL_B} (out of {new["all"] - ALL_B}). {ACTUAL_B} -> {new["actual"]}')
