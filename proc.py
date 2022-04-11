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
from os import mkdir, cpu_count
from os.path import exists
import pandas
from tqdm import tqdm
from multiprocessing import Process, Queue, set_start_method, Value

set_start_method("spawn")

def proc_audio(q, prog, length):
    while not q.empty():
        args = q.get()
        print(f"{prog.value}/{length} ({prog.value/length}) working with {args['path'].split('/')[-1]}")

        # input
        wav, _ = librosa.load(args["path"], sr=8000)
        inc = 0
        # need 6 seconds of audio, so use 48000 overlapping length samples
        for x in range(0, len(wav) - 48000, 36000):
            np.save(
                f"./tmp/x.{args['id']:06d}.{inc:04d}",
                np.array(wav[x : x + 48000], dtype=np.float32),
            )

        # output
        onehot = np.zeros(len(GENRE_TRANSMUTE), dtype=np.float32)
        for genre in args["genres"]:
            onehot[GENRE_TRANSMUTE[genre]] = 1.0
        np.save(f"./tmp/y.{args['id']:06d}", onehot)

        q.task_done()
        prog.value += 1

def main():
    mkdir("./tmp")
    datasheet = pandas.read_csv(argv[2], usecols=["track_id", "track_genres_all"])
    q = Queue()
    prog = Value('i', 0)

    print("gathering files")
    length = 0
    for audiofile in glob(argv[1] + "/*/*.mp3"):
        audio_id = int(audiofile.split("/")[-1].split(".")[0])
        try:
            # skip stuff already done
            np.load(f"./tmp/y.{audio_id:06d}.npy")
        except:
            where = np.where(datasheet["track_id"] == audio_id)[0][0]
            q.put(
                {
                    "path": audiofile,
                    "id": audio_id,
                    "genres": datasheet["track_genres_all"][where][1:-1].split(", "),
                }
            )
            length += 1
    
    print("spinning threads")
    processes = [Process(target=proc_audio, args=(q,prog,length,)) for _ in range(cpu_count())]
    [x.start() for x in processes]
    try:
        [x.join() for x in processes]
    except KeyboardInterrupt as e:
        raise e

    print("concatenating arr")
    amt = len(list(glob("./tmp/x.*")))
    X = np.zeros((amt, 48000), dtype=np.float32)
    np.save("x.npy", X)
    del X
    Y = np.zeros((amt, len(GENRE_TRANSMUTE)), dtype=np.float32)
    np.save("y.npy", y)
    del Y
    X = np.memmap("x.npy", dtype=np.float32, shape=(amt, 48000))
    Y = np.memmap("y.npy", dtype=np.float32, shape=(amt, len(GENRE_TRANSMUTE)))
    pos = 0
    for x_slice_file in glob("./tmp/x.*"):
        x_slice = np.load(x_slice_file)
        y_slice_file = f"./tmp/y.{x_slice_file.split('/')[-1].split('.')[1]}.npy"
        y_slice = np.load(y_slice_file)
        X[pos] = x_slice
        Y[pos] = y_slice
        pos += 1

if __name__ == "__main__":
    main()
