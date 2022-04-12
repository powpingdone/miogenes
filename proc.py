GENRE_TRANSMUTE = {
    "1": 0,
    "2": 1,
    "3": 2,
    "4": 3,
    "5": 4,
    "6": 5,
    "7": 6,
    "8": 7,
    "9": 8,
    "10": 9,
    "11": 10,
    "12": 11,
    "13": 12,
    "14": 13,
    "15": 14,
    "16": 15,
    "17": 16,
    "18": 17,
    "19": 18,
    "20": 19,
    "21": 20,
    "22": 21,
    "25": 22,
    "26": 23,
    "27": 24,
    "30": 25,
    "31": 26,
    "32": 27,
    "33": 28,
    "36": 29,
    "37": 30,
    "38": 31,
    "41": 32,
    "42": 33,
    "43": 34,
    "45": 35,
    "46": 36,
    "47": 37,
    "49": 38,
    "53": 39,
    "58": 40,
    "63": 41,
    "64": 42,
    "65": 43,
    "66": 44,
    "70": 45,
    "71": 46,
    "74": 47,
    "76": 48,
    "77": 49,
    "79": 50,
    "81": 51,
    "83": 52,
    "85": 53,
    "86": 54,
    "88": 55,
    "89": 56,
    "90": 57,
    "92": 58,
    "94": 59,
    "97": 60,
    "98": 61,
    "100": 62,
    "101": 63,
    "102": 64,
    "103": 65,
    "107": 66,
    "109": 67,
    "111": 68,
    "113": 69,
    "117": 70,
    "118": 71,
    "125": 72,
    "130": 73,
    "137": 74,
    "138": 75,
    "166": 76,
    "167": 77,
    "169": 78,
    "170": 79,
    "171": 80,
    "172": 81,
    "173": 82,
    "174": 83,
    "175": 84,
    "176": 85,
    "177": 86,
    "178": 87,
    "179": 88,
    "180": 89,
    "181": 90,
    "182": 91,
    "183": 92,
    "184": 93,
    "185": 94,
    "186": 95,
    "187": 96,
    "188": 97,
    "189": 98,
    "214": 99,
    "224": 100,
    "232": 101,
    "236": 102,
    "240": 103,
    "247": 104,
    "250": 105,
    "267": 106,
    "286": 107,
    "296": 108,
    "297": 109,
    "311": 110,
    "314": 111,
    "322": 112,
    "337": 113,
    "359": 114,
    "360": 115,
    "361": 116,
    "362": 117,
    "374": 118,
    "377": 119,
    "378": 120,
    "400": 121,
    "401": 122,
    "404": 123,
    "428": 124,
    "439": 125,
    "440": 126,
    "441": 127,
    "442": 128,
    "443": 129,
    "444": 130,
    "456": 131,
    "465": 132,
    "468": 133,
    "491": 134,
    "493": 135,
    "495": 136,
    "502": 137,
    "504": 138,
    "514": 139,
    "524": 140,
    "538": 141,
    "539": 142,
    "542": 143,
    "567": 144,
    "580": 145,
    "602": 146,
    "619": 147,
    "651": 148,
    "659": 149,
    "693": 150,
    "695": 151,
    "741": 152,
    "763": 153,
    "808": 154,
    "810": 155,
    "811": 156,
    "906": 157,
    "1032": 158,
    "1060": 159,
    "1156": 160,
    "1193": 161,
    "1235": 162,
}

import librosa
import numpy as np
from glob import iglob as glob
from sys import argv
from os import mkdir
from os.path import exists
import pandas
from tqdm import tqdm
from multiprocessing import Pool, set_start_method
import warnings


def proc_audio(args):
    warnings.filterwarnings(
        "ignore", ".*PySoundFile failed. Trying audioread instead.*"
    )

    # input
    wav, _ = librosa.load(args["path"], sr=8000)
    inc = 0
    # need 6 seconds of audio, so use 48000 overlapping length samples
    for x in range(0, len(wav) - 48000, 36000):
        np.save(
            f"./tmp/x.{args['id']:06d}.{inc:04d}",
            np.array(wav[x : x + 48000], dtype=np.float32),
        )
        inc += 1

    # output
    onehot = np.zeros(len(GENRE_TRANSMUTE), dtype=np.float32)
    for genre in args["genres"]:
        try:        
            onehot[GENRE_TRANSMUTE[genre]] = 1.0
        except KeyError:
            # do nothing
            onehot = onehot
    np.save(f"./tmp/y.{args['id']:06d}", onehot)
    return None


def main():
    set_start_method("spawn")
    if not exists("./tmp"):
        mkdir("./tmp")
    datasheet = pandas.read_csv(argv[2], usecols=["track_id", "track_genres_all"])
    q = []

    print("gathering files")
    length = 0
    for audiofile in glob(argv[1] + "/*/*.mp3"):
        audio_id = int(audiofile.split("/")[-1].split(".")[0])
        try:
            # skip stuff already done
            np.load(f"./tmp/y.{audio_id:06d}.npy")
        except:
            where = np.where(datasheet["track_id"] == audio_id)[0][0]
            q.append(
                {
                    "path": audiofile,
                    "id": audio_id,
                    "genres": datasheet["track_genres_all"][where][1:-1].split(", "),
                }
            )
            length += 1

    print("spinning threads")
    try:
        with Pool() as p:
            list(tqdm(p.imap(proc_audio, q), total=len(q)))
    except KeyboardInterrupt as e:
        raise e

    print("concatenating arr")
    amt = len(list(glob("./tmp/x.*")))
    test_amt = amt // 250
    train_amt = amt - test_amt
    X = np.zeros((train_amt, 48000), dtype=np.float32)
    np.save("x.train.npy", X)
    X = np.zeros((test_amt, 48000), dtype=np.float32)
    np.save("x.test.npy", X)
    del X
    Y = np.zeros((train_amt, len(GENRE_TRANSMUTE)), dtype=np.float32)
    np.save("y.train.npy", Y)
    Y = np.zeros((test_amt, len(GENRE_TRANSMUTE)), dtype=np.float32)
    np.save("y.test.npy", Y)
    del Y
    X_train = np.memmap("x.train.npy", dtype=np.float32, shape=(amt, 48000))
    X_test = np.memmap("x.test.npy", dtype=np.float32, shape=(amt, 48000))
    Y_train = np.memmap("y.train.npy", dtype=np.float32, shape=(amt, len(GENRE_TRANSMUTE)))
    Y_test = np.memmap("y.test.npy", dtype=np.float32, shape=(amt, len(GENRE_TRANSMUTE)))
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
