from tensorflow import keras
from keras import Model, Input
from keras.layers import Conv1D, MaxPooling1D, LSTM, Dense, Flatten, Reshape, Dropout, Add, Activation, Reshape, ZeroPadding1D, BatchNormalization
from constants import *

# inspiration taken from deej-ai
# model used from KMASAHIRO/music2vec

inp_shape = (AUDIO_LEN,1,)

inp = Input(inp_shape)
# block 1
x = ZeroPadding1D(32)(inp)
x = Conv1D(16, 64, 2)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
x = MaxPooling1D(8, 8, padding='valid')(x)
# block 2
x = ZeroPadding1D(16)(x)
x = Conv1D(32, 32, 2)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
x = MaxPooling1D(8, 8, padding='valid')(x)
# block 3
x = ZeroPadding1D(8)(x)
x = Conv1D(64, 16, 2)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 4
x = ZeroPadding1D(4)(x)
x = Conv1D(128, 8, 2)(x)
x = Dropout(0.2)(x)
x = BatchNormalization()(x)
# block res1
res = Add()([Conv1D(128, 8, 1008)(inp), x])
x = Activation('relu')(res)
# block 5
x = ZeroPadding1D(2)(x)
x = Conv1D(256, 4, 2)(x)
x = Dropout(0.1)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
x = MaxPooling1D(4, 4)(x)
# block 6
x = ZeroPadding1D(2)(x)
x = Conv1D(512, 4, 2)(x)
x = Dropout(0.1)(x)
x = BatchNormalization()(x)
x = Activation('relu')(x)
# block 7
x = ZeroPadding1D(2)(x)
x = Conv1D(1024, 4, 2)(x)
x = Dropout(0.1)(x)
x = BatchNormalization()(x)
# block res2
res = Add()([Conv1D(1024, 4, 16384)(res), x])
x = Activation('relu')(res)
# block LSTM
x = Dropout(0.1)(x)
x = LSTM(512, return_sequences=True)(x)
x = LSTM(512, return_sequences=True)(x)
x = Flatten()(x)
x = Dropout(0.1)(x)
# block DENSE
x = Dense(1024)(x)
x = Dropout(0.2)(x)
x = Activation('relu')(x)
x = Dense(1024)(x)
x = Dropout(0.2)(x)
res = Add()([Dense(1024)(Flatten()(res)), x])
x = Activation('relu')(res)
x = Dense(256)(x)
x = Activation('relu')(x)
x = Dropout(0.1)(x)
out = Dense(GENRE_AMT, activation='sigmoid')(x)

model = Model(inputs=inp, outputs=out)

model.compile(
    optimizer="adam",
    loss="binary_crossentropy",
    metrics=["accuracy"]
)

model.summary()
model.save("model.tf_000")

