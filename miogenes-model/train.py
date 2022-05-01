from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob

from constants import *

def gener_tr(arr):
    while True:
        for x in range(0,int(len(arr)*.9)):
            yield (arr[x], arr[x],)

def gener_te(arr):
    while True:
        for x in range(int(len(arr)*.9)):
            yield (arr[x], arr[x],)

arr = np.memmap("train.npy", dtype=np.float32, mode="r")
arr = arr.reshape((len(arr) // AUDIO_LEN, AUDIO_LEN))

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

autoenc.fit(
    gener_tr(arr),
    batch_size=32,
    epochs=10,
    shuffle=True,
    validation_data=gener_te(arr),
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{val_loss:.5f}"),
        EarlyStopping(patience=6, min_delta=0.01, monitor="val_loss"),
    ],
)
