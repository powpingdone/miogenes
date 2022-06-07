from tensorflow import keras
from keras import Model, Input
from keras.layers import Conv1D, MaxPooling1D, LSTM, Dense, Flatten, Reshape, Dropout, Add, Activation, Reshape, ZeroPadding1D, BatchNormalization
from constants import *

# inspiration taken from deej-ai

inp_shape = (AUDIO_LEN,1,)

inp = Input(inp_shape)
# block 1
x = Conv1D(32, 64, 4)(inp)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 2
x = Conv1D(64, 32, 4)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 3
x = Conv1D(128, 16, 4)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 4
x = Conv1D(256, 8, 2)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
# block 5
x = Conv1D(512, 4, 2)(x)
x = Dropout(0.1)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 6
x = Conv1D(1024, 4, 2)(x)
x = Dropout(0.1)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block LSTM
x = Dropout(0.1)(x)
x = LSTM(512, return_sequences=True)(x)
x = LSTM(256, return_sequences=True)(x)
x = LSTM(256)(x)
x = Dropout(0.1)(x)
# block DENSE
x = Dense(256)(x)
x = Dropout(0.2)(x)
x = Activation('relu')(x)
x = Dense(128)(x)
x = Activation('relu')(x)
x = Dropout(0.1)(x)
out = Dense(GENRE_AMT, activation='sigmoid')(x)

model = Model(inputs=inp, outputs=out)

model.compile(
    optimizer=keras.optimizers.Adadelta(learning_rate=1),
    loss="binary_crossentropy",
    metrics=["accuracy"],
    jit_compile=True
)

model.summary()
model.save("model_000")

