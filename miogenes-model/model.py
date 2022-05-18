from keras import optimizers
from plaidml import keras
import keras.backend as K
from keras.models import Model
from keras.layers import Input, Conv1D as CL, AveragePooling1D as down, UpSampling1D as up

from constants import *

encinp = Input((AUDIO_LEN,1))
enc = CL(128, 32, strides=16, padding="same")(encinp)
enc = CL(256, 16, strides=16, padding="same")(enc)
enc = CL(1, 1, padding="same", activation="sigmoid")(enc)
next_shape = Model(encinp, enc).output_shape[1:]

decinp = Input(next_shape)
dec = CL(1, 1, padding="same")(decinp)
dec = up(16)(dec)
dec = CL(256, 16, padding="same")(dec)
dec = up(16)(dec)
dec = CL(128, 32, padding="same")(dec)
dec = CL(1,1, padding="same")(dec)

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
