from tensorflow import keras
from keras.models import Model
from keras.layers import Input, Dense

from constants import *

encinp = Input((AUDIO_LEN,))
enc = Dense(12000, activation='relu')(encinp)
enc = Dense(4096, activation='relu')(enc)
enc = Dense(512, activation='relu')(enc)
enc = Dense(128)(enc)

decinp = Input((128,))
dec = Dense(512, activation='relu')(decinp)
dec = Dense(4096, activation='relu')(dec)
dec = Dense(12000, activation='relu')(dec)
dec = Dense(AUDIO_LEN)(dec)

inp = Input(
    (AUDIO_LEN,),
    name="fullinp",
)
encoder = Model(encinp, enc, name="enc")(inp)
decoder = Model(decinp, dec, name="dec")(encoder)
autoenc = Model(inp, decoder)

autoenc.compile(
    optimizer=keras.optimizers.Adadelta(1),
    loss="mean_squared_error",
)
autoenc.summary()

autoenc.save("model_000")
