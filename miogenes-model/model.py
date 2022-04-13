from tensorflow import keras
from keras import Sequential, Input
from keras.layers import Conv1D, MaxPooling1D, LSTM, Dense, Flatten, Softmax, Reshape
from constants import *

inp_shape = (AUDIO_LEN,1,)

model = Sequential(
    [
        Input(inp_shape),
        Conv1D(16, 64, 8),
        MaxPooling1D(8, 2),
        Conv1D(32, 32, 8),
        MaxPooling1D(8, 2),
        Conv1D(64, 16, 2),
        Conv1D(128, 8, 2),
        Conv1D(256, 4, 2),
        MaxPooling1D(4, 2),
        Conv1D(512, 4, 1),
        Conv1D(1024, 4, 1),
        LSTM(1024, return_sequences=True),
        LSTM(1024),
        Flatten(),
        Dense(1024),
        Dense(512),
        Dense(len(GENRE_TRANSMUTE)),
        Softmax()
    ]
)

model.compile(
    optimizer="adadelta", 
    loss="categorical_crossentropy",
    metrics=["accuracy"]
)

model.summary()
model.save("model.tf")

