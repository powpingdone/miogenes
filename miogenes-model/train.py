from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob
from gc import collect
from random import shuffle, seed
from constants import *
from model import encoder, decoder, model_build

BATCH_SIZE = 32
seed(58790032)

def gener_tr(files):
    while True:
        shuffle(files)
        for x in range(0, len(files) - BATCH_SIZE, BATCH_SIZE):
            lis = []
            for pos in range(x, x + BATCH_SIZE):
                lis += [np.load(files[pos])]
            outx = np.asarray(lis).reshape(BATCH_SIZE, AUDIO_LEN)
            yield (
                outx,
                outx,
            )
            collect()

files = glob("samples/*.npy")
shuffle(files)
train = files[:int(0.9*len(files))]
test = files[int(0.9*len(files)) + 1:]

class AutoEncSave(keras.callbacks.Callback):
    def __init__(self, enc, dec):
        super().__init__()
        self.enc = enc
        self.dec = dec

    def on_epoch_end(self, epoch, logs=None):
        self.enc.save(f"model_enc_{epoch:03d}_{logs['loss']:.6f}_{logs['val_loss']:.6f}")
        self.dec.save(f"model_dec_{epoch:03d}_{logs['loss']:.6f}_{logs['val_loss']:.6f}")

encoder = encoder()
decoder = decoder()
model = model_build(encoder, decoder)

initial_epoch = glob("model_enc_*")
initial_epoch.sort()
initial_epoch = int(initial_epoch[-1].split("_")[2]) + 1 if len(initial_epoch) != 0 else 0

callback = [
    AutoEncSave(encoder, decoder),
    EarlyStopping(patience=6, min_delta=0.01, monitor="val_loss"),
]

model.fit(
    gener_tr(train),
    steps_per_epoch=len(train) // BATCH_SIZE,
    epochs=80,
    initial_epoch=initial_epoch,
    callbacks=callback,
    validation_data=gener_tr(test),
    validation_steps=len(test) // BATCH_SIZE,
)
