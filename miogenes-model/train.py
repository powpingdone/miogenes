from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob

from constants import *

train = np.memmap("train.npy", dtype=np.float32, mode="r")
train = train.reshape((len(train) // AUDIO_LEN, AUDIO_LEN))

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

autoenc.fit(
    train,
    train,
    batch_size=64,
    epochs=10,
    shuffle=True,
    validation_split=0.1,
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{val_loss:.5f}"),
        EarlyStopping(patience=6, min_delta=0.01, monitor="val_loss"),
    ],
)
