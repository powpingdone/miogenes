from tensorflow import keras
from keras.models import Model
from keras.layers import Input, Conv1D, Conv1DTranspose, Dense, Flatten, Reshape

from constants import *

KER = 12
STRID = 4

encinp = Input((AUDIO_LEN, 1))
convenc = Conv1D(256, KER, STRID, padding="same", activation="relu")(encinp)
convenc = Conv1D(32, KER, STRID, padding="same", activation="relu")(convenc)
convenc = Conv1D(4, KER, STRID, padding="same", activation="relu")(convenc)
resh = Model(encinp, convenc).output_shape[1:]
enc = Flatten()(convenc)
enc = Dense(256, activation="relu")(enc)
enc = Dense(128)(enc)


decinp = Input((128,))
dec = Dense(256, activation="relu")(decinp)
dec = Dense(resh[0] * resh[1], activation="relu")(dec)
dec = Reshape(resh)(dec)
convdec = Conv1DTranspose(4, KER, STRID, padding="same", activation="relu")(dec)
convdec = Conv1DTranspose(32, KER, STRID, padding="same", activation="relu")(convdec)
convdec = Conv1DTranspose(256, KER, STRID, padding="same", activation="relu")(convdec)
dec = Conv1DTranspose(1, KER, padding="same")(convdec)

inp = Input(
    (AUDIO_LEN,),
    name="fullinp",
)
encoder = Model(encinp, enc, name="enc")
encoder.summary()
encoder = encoder(inp)
decoder = Model(decinp, dec, name="dec")
decoder.summary()
decoder = decoder(encoder)
autoenc = Model(inp, decoder)

autoenc.compile(
    optimizer=keras.optimizers.Adadelta(1),
    loss="mean_squared_error",
)
autoenc.summary()

autoenc.save("model_000")
