from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, TensorBoard, EarlyStopping
import numpy as np

from constants import *

X_train = np.memmap("x.train.npy", dtype=np.float32, mode="r")
X_train = X_train.reshape((len(X_train) // AUDIO_LEN, AUDIO_LEN))
Y_train = np.memmap("y.train.npy", dtype=np.float32, mode="r")
Y_train = Y_train.reshape((len(Y_train) // len(GENRE_TRANSMUTE), len(GENRE_TRANSMUTE)))
X_test = np.memmap("x.test.npy", dtype=np.float32, mode="r")
X_test = X_test.reshape((len(X_test) // AUDIO_LEN, AUDIO_LEN))
Y_test = np.memmap("y.test.npy", dtype=np.float32, mode="r")
Y_test = Y_test.reshape((len(Y_test) // len(GENRE_TRANSMUTE), len(GENRE_TRANSMUTE)))

model = load_model("model.tf")
callback = [
    ModelCheckpoint("models"),
    TensorBoard(),
    EarlyStopping(patience=3),
]

model.fit(
    X_train,
    Y_train,
    batch_size=128,
    epochs=40,
    shuffle=True,
    callbacks=callback,
    validation_data=(X_test, Y_test),
)
