from tensorflow import keras
from keras.models import Model
from keras.layers import Input, Dense

from constants import *

LATENT = 128

encinp = Input((AUDIO_LEN,))
enc = Dense(4096, activation="relu")(encinp)
enc = Dense(256, activation="relu")(enc)
enc = Dense(LATENT, activation="sigmoid")(enc)


decinp = Input((LATENT,))
dec = Dense(256, activation="relu")(decinp)
dec = Dense(4096, activation="relu")(dec)
dec = Dense(AUDIO_LEN, activation="relu")(dec)

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
