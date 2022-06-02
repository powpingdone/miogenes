import tensorflow.keras as keras
from keras.layers import Input
from keras.models import load_model, Model
from keras.callbacks import ModelCheckpoint, EarlyStopping
from tensorflow.keras.optimizers import Adadelta
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

chooseenc = glob("model_enc_*")
chooseenc.sort()
chooseenc = chooseenc[-1]
print(f"loading model {chooseenc}")
encoder = load_model(chooseenc)

choosedec = glob("model_dec_*")
choosedec.sort()
choosedec = choosedec[-1]
print(f"loading model {choosedec}")
decoder = load_model(choosedec)

inp = Input(
    (AUDIO_LEN,1),
    name="fullinp",
)
encoder = encoder(inp)
decoder = decoder(encoder)
autoenc = Model(inp, decoder)
autoenc.compile(
    optimizer=Adadelta(learning_rate=1),
    loss="mean_absolute_error",
    jit_compile=True,
)

class SaveEnc(keras.callbacks.Callback):
    def on_epoch_end(self, epoch, logs=None):
        encoder.save(f"model_enc_{epoch:03d}")
        decoder.save(f"model_dec_{epoch:03d}")

autoenc.fit(
    gener_tr(files),
    steps_per_epoch=len(files) // 16 // BATCH_SIZE,
    epochs=40,
    initial_epoch=int(chooseenc.split("_")[-1].split("-")[0]),
    callbacks=[
        EarlyStopping(patience=3, min_delta=0.01, monitor="loss"),
    ],
)
