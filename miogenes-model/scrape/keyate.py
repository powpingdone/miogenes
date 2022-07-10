PAIRS = [
#   adding keys in form of:
#   ['IDIDIDIDIDIDIDIDIDIDIDIDIDIDIDID', 'SECRETSECRETSECRETSECRETSECRETSE']
]

from subprocess import run

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
    for (e, i) in enumerate(PAIRS):
        ID = i[0]
        SECRET = i[1]
        print(f'key {e + 1} out of {len(PAIRS)}')
        run(['./target/release/scrape'], env={"RSPOTIFY_CLIENT_ID": ID, "RSPOTIFY_CLIENT_SECRET": SECRET})
except KeyboardInterrupt:
    check = check

new = getlens()
print()
print(f'new playlists: {new["actual"] - ACTUAL_B} (out of {new["all"] - ALL_B}). {ACTUAL_B} -> {new["actual"]}')
