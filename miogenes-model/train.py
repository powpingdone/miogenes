from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob

from constants import *

BATCH_SIZE = 32

def gener_tr(arr):
    global BATCH_SIZE
    while True:
        for x in range(0, int(len(arr)*.9), BATCH_SIZE):
            lis = []
            for pos in range(x, x + BATCH_SIZE):
                lis += [arr[pos]]
            yield (np.asarray(lis), np.asarray(lis),)

arr = np.memmap("train.npy", dtype=np.float32, mode="r")
arr = arr.reshape((len(arr) // AUDIO_LEN, AUDIO_LEN))
val_arr = arr[int(len(arr)*0.9):]

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

autoenc.fit(
    gener_tr(arr),
    steps_per_epoch=len(arr) // BATCH_SIZE,
    epochs=10,
    shuffle=True,
    validation_data=(val_arr, val_arr,),
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{val_loss:.5f}"),
        EarlyStopping(patience=6, min_delta=0.01, monitor="val_loss"),
    ],
    use_multiprocessing=True,
)
