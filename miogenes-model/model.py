import tensorflow.keras
from tensorflow.keras.optimizers.experimental import Adadelta
from keras.models import Model
from keras.layers import Input, Conv1D as CL, UpSampling1D as up

from constants import *

FILTERS = 2048
KERNEL = 128
INTERNAL_NEURONS = 50

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
    optimizer=Adadelta(learning_rate=1),
    loss="mean_absolute_error",
    jit_compile=True,
)
autoenc.summary()

autoenc.save("model_000")
