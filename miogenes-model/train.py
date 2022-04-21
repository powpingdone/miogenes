from tensorflow import keras
from keras.models import load_model
from keras.callbacks import ModelCheckpoint, EarlyStopping
import numpy as np
from glob import glob

from constants import *

X_train = np.memmap("x.train.npy", dtype=np.float32, mode="r")
X_train = X_train.reshape((len(X_train) // AUDIO_LEN, AUDIO_LEN))
Y_train = np.memmap("y.train.npy", dtype=np.float32, mode="r")
Y_train = Y_train.reshape((len(Y_train) // GENRE_AMT, GENRE_AMT))
X_test = np.memmap("x.test.npy", dtype=np.float32, mode="r")
X_test = X_test.reshape((len(X_test) // AUDIO_LEN, AUDIO_LEN))
Y_test = np.memmap("y.test.npy", dtype=np.float32, mode="r")
Y_test = Y_test.reshape((len(Y_test) // GENRE_AMT, GENRE_AMT))

choose = glob("model.tf_*")
choose.sort()
choose = choose[-1]
print(f'loading model {choose}')
model = load_model(choose)
callback = [
    ModelCheckpoint("model.tf_{epoch:02d}_{loss:4f}_{accuracy:.3f}"),
    EarlyStopping(patience=6, min_delta=0.01, monitor="val_accuracy"),
]

model.fit(
    X_train,
    Y_train,
    batch_size=64,
    epochs=40,
    shuffle=True,
    callbacks=callback,
    validation_data=(X_test, Y_test),
)
