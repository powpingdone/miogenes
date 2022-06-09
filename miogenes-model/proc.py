from copy import deepcopy as copy
from glob import iglob as glob
import pandas
from multiprocessing import Pool, set_start_method
import numpy as np
from os import mkdir, remove
from os.path import exists
from random import shuffle
import soundfile as sf
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
    # input
    # try to load the file
    wavpath = "/tmp/" + str(args["id"]) + ".wav"
    conv = Popen(
        ["ffmpeg", "-y", "-i", args["path"], "-ac", "1", "-ar", str(SAMPLING), wavpath],
        stdout=DEVNULL,
        stderr=DEVNULL,
    )
    conv.wait()
    try:
        wav, _ = sf.read(wavpath)
    except Exception as e:
        print(args["path"])
        raise e
    conv = Popen(["rm", wavpath])

    # preprocess the wavform
    ptp = np.ptp(wav)
    if ptp == 0:
        return None
    wav = (wav - np.min(wav)) / ptp
    if np.isnan(wav).any():
        return None

    # write out the temp arrays
    inc = 0
    for x in range(0, len(wav) - AUDIO_LEN, int(SAMPLING * 3.5)):
        np.save(
            f"./samples/{args['id']:06d}.{inc:04d}",
            np.array(wav[x : x + AUDIO_LEN], dtype=np.float32),
        )
        inc += 1
    conv.wait()
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
    for dirpath in argv[1:]:
        for audiofile in glob(dirpath + "/*.*"):
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
    q = []
    with open("bad_ids.txt") as x:
        bad = {int(r): None for r in x.readlines()}
    for audiofile in tqdm(
        glob(argv[1] + "/*.*"), total=len(list(glob(argv[1] + "/*.*")))
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

    print("proccessing all files")
    try:
        with Pool() as p:
            list(tqdm(p.imap(proc_audio, q), total=len(q)))
    except KeyboardInterrupt as e:
        raise e


if __name__ == "__main__":
    main()
