from copy import deepcopy as copy
from glob import iglob as glob
import pandas
import librosa
from multiprocessing import Pool, set_start_method
import numpy as np
from os import mkdir, remove
from os.path import exists
from random import shuffle
from subprocess import Popen, DEVNULL
from sys import argv
from tqdm import tqdm
import warnings

from constants import *


def validate_file(args):
    # check if audiofile is valid
    # if this has a invalid returncode, then ffmpeg failed to decode
    probe = Popen(
        ["ffmpeg", "-i", args["path"], "-f", "null", "-"],
        stdout=DEVNULL,
        stderr=DEVNULL,
    )
    probe.wait()
    if probe.returncode != 0:
        print(
            f"ffmpeg doesn't like ID {args['id']:06d}, "
            "so it must be invalid. Caching as a bad id."
        )
        return {"good": False, "id": args["id"]}
    return {"good": True, "id": args["id"]}


def proc_audio(args):
    # we just want the signal dammit, we don't care about the
    # backend being used to get the signal
    warnings.filterwarnings(
        "ignore", ".*PySoundFile failed. Trying audioread instead.*"
    )

    # input
    # try to load the file
    try:
        wav, _ = librosa.load(args["path"], sr=SAMPLING, mono=True, dtype=np.float32)
    except Exception as e:
        print(args["path"])
        raise e
    ptp = np.ptp(wav)

    # write out the temp arrays
    if ptp != 0:
        wav = (wav - np.min(wav)) / ptp
        if np.isnan(wav).any():
            return None
        inc = 0
        if len(wav) / SAMPLING < CAPTURE_LEN:  # too small for use
            return None
        elif len(wav) / SAMPLING > 30:  # typical song
            bases = [0, 1 / 6, 1 / 4, 1 / 3, 1 / 2, 2 / 3, 3 / 4, 5 / 6]
        else:  # 30 second sample
            bases = [0, 1 / 4, 1 / 3, 1 / 2, 2 / 3]
        sbases = [int(len(wav) * x) for x in bases]
        for x in sbases:
            if x + AUDIO_LEN < len(wav):
                np.save(
                    f'./samples/{args["id"]:06d}.{inc:01d}',
                    np.array(wav[x : x + AUDIO_LEN], dtype=np.float32),
                )
                inc += 1

    return None


def main():
    set_start_method("spawn")
    if not exists("./samples"):
        mkdir("./samples")

    print("validating files, if this goes by quickly then it used all cached files")
    good = {}
    bad = {}
    q = []
    if exists("good_ids.txt") and exists("bad_ids.txt"):
        with open("good_ids.txt") as x:
            good = {int(r): None for r in x.readlines()}
        with open("bad_ids.txt") as x:
            bad = {int(r): None for r in x.readlines()}
    for path in argv[1:]:
        for audiofile in glob(path + "/*"):
            audio_id = int(audiofile.split("/")[-1].split(".")[0])
            if not (audio_id in good or audio_id in bad):
                q.append(
                    copy(
                        {
                            "path": audiofile,
                            "id": audio_id,
                        }
                    )
                )
    try:
        with Pool() as p, open("good_ids.txt", "a+") as good, open(
            "bad_ids.txt", "a+"
        ) as bad:
            for potent in tqdm(p.imap(validate_file, q), total=len(q)):
                if potent["good"]:
                    good.write(f'{potent["id"]}\n')
                else:
                    bad.write(f'{potent["id"]}\n')
    except KeyboardInterrupt as e:
        raise e

    print("gathering files")
    length = 0
    q = []
    with open("bad_ids.txt") as x:
        bad = {int(r): None for r in x.readlines()}
    for path in argv[1:]:
        print(f"PATH: {path}")
        for audiofile in tqdm(
            glob(path + "/*"), total=len(list(glob(path + "/*")))
        ):
            audio_id = int(audiofile.split("/")[-1].split(".")[0])
            if audio_id in bad:
                continue  # this isn't a valid file

            q.append(
                copy(
                    {
                        "path": audiofile,
                        "id": audio_id,
                    }
                )
            )
            length += 1

    print("proccessing all files")
    try:
        with Pool() as p:
            list(tqdm(p.imap(proc_audio, q), total=len(q)))
    except KeyboardInterrupt as e:
        raise e


if __name__ == "__main__":
    main()
