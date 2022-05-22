from keras import optimizers
from plaidml import keras
import keras.backend as K
from keras.models import Model
from keras.layers import Input, Conv1D as CL, AveragePooling1D as down, UpSampling1D as up, LSTM as LS

from constants import *

FILTERS = 2048
KERNEL = 512
INTERNAL_NEURONS = 40

encinp = Input((AUDIO_LEN,1))
enc = CL(FILTERS, KERNEL, strides=AUDIO_LEN // INTERNAL_NEURONS, padding="same")(encinp)
enc = CL(1, 1, padding="same")(enc)
next_shape = Model(encinp, enc).output_shape[1:]

decinp = Input(next_shape)
dec = up(AUDIO_LEN // INTERNAL_NEURONS)(decinp)
dec = CL(FILTERS, KERNEL, padding="same")(dec)
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
    loss="mean_absolute_error",
)
autoenc.summary()

autoenc.save("model_000")
