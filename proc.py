from copy import deepcopy as copy
from glob import iglob as glob
import pandas
import librosa
from multiprocessing import Pool, set_start_method
import numpy as np
from os import mkdir
from os.path import exists
from subprocess import Popen, DEVNULL
from sys import argv
from tqdm import tqdm
import warnings

from constants import *


def validate_file(args):
    # check if audiofile is valid
    # if this has a invalid returncode, then ffmpeg failed to decode
    probe = Popen(["ffmpeg", "-i", args["path"], "-f", "null", "-"], stdout=DEVNULL, stderr=DEVNULL)
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
    try:
        wav, _ = librosa.load(args["path"], sr=SAMPLING)
    except Exception as e:
        print(args["path"])
        raise e
    inc = 0
    # 4 second steps
    for x in range(0, len(wav) - AUDIO_LEN, 32000):
        np.save(
            f"./tmp/x.{args['id']:06d}.{inc:04d}",
            np.array(wav[x : x + AUDIO_LEN], dtype=np.float32),
        )
        inc += 1

    # output
    onehot = np.zeros(len(GENRE_TRANSMUTE), dtype=np.float32)
    for genre in args["genres"]:
        onehot[genre] = 1.0
    np.save(f"./tmp/y.{args['id']:06d}", onehot)
    return None


def main():
    set_start_method("spawn")
    if not exists("./tmp"):
        mkdir("./tmp")
    datasheet = pandas.read_csv(argv[2], usecols=["track_id", "track_genres_all"])

    print("validating files, if this goes by quickly then it used all cached files")
    good = {}
    bad = {}
    q = []
    if exists("good_ids.txt") and exists("bad_ids.txt"):
        with open("good_ids.txt") as x:
            good = {int(r): None for r in x.readlines()}
        with open("bad_ids.txt") as x:
            bad = {int(r): None for r in x.readlines()}
    for audiofile in glob(argv[1] + "/*/*.mp3"):
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
    for audiofile in tqdm(
        glob(argv[1] + "/*/*.mp3"), total=len(list(glob(argv[1] + "/*/*.mp3")))
    ):
        audio_id = int(audiofile.split("/")[-1].split(".")[0])
        if audio_id in bad:
            continue  # this isn't a valid file

        try:
            # skip stuff already done
            np.load(f"./tmp/y.{audio_id:06d}.npy")
        except:
            where = np.where(datasheet["track_id"] == audio_id)[0][0]
            unproc_genres = datasheet["track_genres_all"][where][1:-1].split(", ")
            genres = []
            invalid = False
            # genres doesn't seem to work all that often, lets make sure it works
            # so that we can skip work if it doesn't work
            for val in unproc_genres:
                if val in GENRE_TRANSMUTE:
                    genres.append(GENRE_TRANSMUTE[val])
                elif val == "":
                    continue
                else:
                    invalid = True
                    break
            if invalid:
                print(
                    f"ID {audio_id} does not have a valid genre, skipping. ",
                    f"Was given for input \"{datasheet['track_genres_all'][where]}\"",
                )
                continue

            q.append(
                copy(
                    {
                        "path": audiofile,
                        "id": audio_id,
                        "genres": genres,
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

    print("concatenating full dataset")
    amt = len(list(glob("./tmp/x.*")))
    test_amt = amt // 250
    train_amt = amt - test_amt
    X = np.zeros((train_amt, AUDIO_LEN), dtype=np.float32)
    np.save("x.train.npy", X)
    X = np.zeros((test_amt, AUDIO_LEN), dtype=np.float32)
    np.save("x.test.npy", X)
    del X
    Y = np.zeros((train_amt, len(GENRE_TRANSMUTE)), dtype=np.float32)
    np.save("y.train.npy", Y)
    Y = np.zeros((test_amt, len(GENRE_TRANSMUTE)), dtype=np.float32)
    np.save("y.test.npy", Y)
    del Y
    X_train = np.memmap("x.train.npy", dtype=np.float32, shape=(amt, AUDIO_LEN))
    X_test = np.memmap("x.test.npy", dtype=np.float32, shape=(amt, AUDIO_LEN))
    Y_train = np.memmap(
        "y.train.npy", dtype=np.float32, shape=(amt, len(GENRE_TRANSMUTE))
    )
    Y_test = np.memmap(
        "y.test.npy", dtype=np.float32, shape=(amt, len(GENRE_TRANSMUTE))
    )
    pos = 0
    for x_slice_file in tqdm(glob("./tmp/x.*"), total=amt):
        x_slice = np.load(x_slice_file)
        y_slice_file = f"./tmp/y.{x_slice_file.split('/')[-1].split('.')[1]}.npy"
        y_slice = np.load(y_slice_file)
        if pos % 250 == 0:
            X_test[pos] = x_slice
            Y_test[pos] = y_slice
        else:
            X_train[pos] = x_slice
            Y_train[pos] = y_slice
        pos += 1


if __name__ == "__main__":
    main()