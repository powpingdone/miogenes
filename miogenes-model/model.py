from keras import optimizers
from plaidml import keras
import keras.backend as K
from keras.models import Model
from keras.layers import Input, Conv1D, MaxPooling1D, UpSampling1D, Flatten

from constants import *

encinp = Input((AUDIO_LEN, 1))
enc = Conv1D(512, 256, padding="same", activation="relu")(encinp)
enc = MaxPooling1D(64)(enc)
enc = Conv1D(1, 1, strides=8, padding="same", activation="relu")(enc)
next_shape = Model(encinp, enc).output_shape[1:]

decinp = Input(next_shape)
dec = Conv1D(1, 1, padding="same", activation="relu")(decinp)
dec = UpSampling1D(8)(dec)
dec = Conv1D(512, 256, padding="same", activation="relu")(dec)
dec = UpSampling1D(64)(dec)
dec = Conv1D(1, 1, padding="same", activation="sigmoid")(dec)

inp = Input(
    (AUDIO_LEN,1),
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
    optimizer=optimizers.Adadelta(1),
    loss="mean_squared_error",
)
autoenc.summary()

autoenc.save("model_000")
