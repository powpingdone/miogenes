from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob
from gc import collect
from random import shuffle, seed
from constants import *

BATCH_SIZE = 32
seed(58790032)

def gener_tr(files):
    while True:
        shuffle(files)
        for x in range(0, len(files) - BATCH_SIZE, BATCH_SIZE):
            lis = []
            oue = []
            for pos in range(x, x + BATCH_SIZE):
                lis += [np.load(files[pos])]
                oue += [np.load(f'y/y.{int(files[pos].split(".")[1]):06d}.npy')]
            outx = np.asarray(lis).reshape(BATCH_SIZE, AUDIO_LEN, 1)
            outy = np.asarray(oue).reshape(BATCH_SIZE, GENRE_AMT, 1)
            yield (
                outx,
                outy,
            )
            collect()

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f'loading model {choose}')
model = load_model(choose)
callback = [
    ModelCheckpoint("model_{epoch:02d}_{loss:.6f}_{accuracy:.3f}_{val_accuracy:.3f}"),
    EarlyStopping(patience=6, min_delta=0.01, monitor="val_accuracy"),
]
files = glob("x/*.npy")
shuffle(files)
train = files[:int(0.9*len(files))]
test = files[int(0.9*len(files)) + 1:]

model.fit(
    gener_tr(train),
    steps_per_epoch=len(train) // BATCH_SIZE,
    epochs=80,
    initial_epoch=int(choose.split("_")[1]),
    callbacks=callback,
    validation_data=gener_tr(test),
    validation_steps=len(test) // BATCH_SIZE
)
