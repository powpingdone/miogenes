from tensorflow import keras
from keras.models import Model
from keras.layers import (
    Conv1D,
    MaxPooling1D,
    Input,
    UpSampling1D,
    BatchNormalization,
    LeakyReLU,
)

from constants import *

encinp = Input(
    (
        AUDIO_LEN,
        1,
    )
)
enc = BatchNormalization()(encinp)
enc = Conv1D(AUDIO_LEN // 512, 64, activation="relu", padding="same")(enc)
enc = MaxPooling1D(8, 8, padding="same")(enc)
enc = Conv1D(4096, 16, activation="relu", padding="same")(enc)
enc = MaxPooling1D(4, 4, padding="same")(enc)
enc = Conv1D(128, 8, activation="relu", padding="same")(enc)
enc = MaxPooling1D(4, 4, padding="same")(enc)

decinp = Input(
    (
        375,
        128,
    )
)
dec = Conv1D(128, 8, activation="relu", padding="same")(decinp)
dec = UpSampling1D(4 * 4)(dec)
dec = Conv1D(4096, 16, activation="relu", padding="same")(dec)
dec = UpSampling1D(4 * 4)(dec)
dec = Conv1D(AUDIO_LEN // 512, 64, activation="relu", padding="same")(dec)
dec = MaxPooling1D(2, 2, padding="same")(dec)
dec = Conv1D(1, 16, padding="same")(dec)

inp = Input(
    (
        AUDIO_LEN,
        1,
    ),
    name="fullinp",
)
encoder = Model(encinp, enc, name="enc")(inp)
decoder = Model(decinp, dec, name="dec")(encoder)
autoenc = Model(inp, decoder)

autoenc.compile(
    optimizer=keras.optimizers.Adadelta(1),
    loss="binary_crossentropy",
    metrics=["accuracy"],
)
autoenc.summary()

autoenc.save("model_000")
