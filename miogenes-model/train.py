from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob

from constants import *

train = np.memmap("train.npy", dtype=np.float32, mode="r")
train = train.reshape((len(train) // AUDIO_LEN, AUDIO_LEN))
train_out = np.memmap("train.npy", dtype=np.float32, mode="r")
train_out = train_out.reshape((len(train) // AUDIO_LEN, AUDIO_LEN))
test = np.memmap("test.npy", dtype=np.float32, mode="r")
test = test.reshape((len(test) // AUDIO_LEN, AUDIO_LEN))
test_out = np.memmap("test.npy", dtype=np.float32, mode="r")
test_out = test_out.reshape((len(test) // AUDIO_LEN, AUDIO_LEN))

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

autoenc.fit(
    train,
    train_out,
    batch_size=64,
    epochs=40,
    shuffle=True,
    validation_data=[test, test_out],
    callbacks=[
        ModelCheckpoint("model_{epoch:03d}-{val_accuracy:.2f}.h5"),
        EarlyStopping(patience=6, min_delta=0.01, monitor="val_accuracy"),
    ],
)
