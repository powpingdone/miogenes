import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob
from random import shuffle, seed
from gc import collect

from constants import *

seed(8769321)

BATCH_SIZE = 16

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

autoenc.fit_generator(
    gener_tr(files),
    steps_per_epoch=len(files) // BATCH_SIZE,
    epochs=40,
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{loss:.5f}"),
        EarlyStopping(patience=6, min_delta=0.01, monitor="loss"),
    ],
)
