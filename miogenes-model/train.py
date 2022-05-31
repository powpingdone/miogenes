import tensorflow.keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob
from random import shuffle
from gc import collect

from constants import *

BATCH_SIZE = 1

def gener_tr(files):
    while True:
        shuffle(files)
        for x in range(0, len(files) - BATCH_SIZE, BATCH_SIZE):
            lis = []
            for pos in range(x, x + BATCH_SIZE):
                lis += [np.load(files[pos])]
            out = np.asarray(lis).reshape(BATCH_SIZE, AUDIO_LEN, 1)
            yield (
                out,
                out,
            )
            collect()


files = glob("./samples/*.npy")
files.sort()
shuffle(files)

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

autoenc.fit(
    gener_tr(files),
    steps_per_epoch=len(files) // 16 // BATCH_SIZE,
    epochs=40,
    initial_epoch=int(choose.split("_")[-1].split("-")[0]),
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{loss:.6f}"),
        EarlyStopping(patience=3, min_delta=0.01, monitor="loss"),
    ],
)
