from sys import exit
from constants import *

SIZE = 5

if AUDIO_LEN % (SIZE ** 3) != 0:
    print(f"invalid size: {AUDIO_LEN / (SIZE ** 3)}")
    while AUDIO_LEN % (SIZE ** 3) != 0 and (SIZE ** 3) < AUDIO_LEN:
        SIZE += 1
    if (SIZE ** 3) >= AUDIO_LEN:
        print("cannot find a multiple close")
    else:
        print(f"next closest is {SIZE}")
    
    exit(1)

import tensorflow.keras
from tensorflow.keras.optimizers.experimental import Adadelta
from keras.models import Model
from keras.layers import Input, Conv1D as CL, UpSampling1D as up

STRIDES = AUDIO_LEN // SIZE

encinp = Input((AUDIO_LEN,1))
enc = CL(2048, 16, strides=STRIDES, padding="same")(encinp)
enc = CL(1024, 64, strides=STRIDES, padding="same")(encinp)
enc = CL(256, 128, strides=STRIDES, padding="same")(encinp)
enc = CL(1, 1, padding="same")(enc)
next_shape = Model(encinp, enc).output_shape[1:]

decinp = Input(next_shape)
dec = up(STRIDES)(decinp)
dec = CL(256, 128, padding="same")(dec)
dec = up(STRIDES)(dec)
dec = CL(1024, 64, padding="same")(dec)
dec = up(STRIDES)(dec)
dec = CL(2048, 16, padding="same")(dec)
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
