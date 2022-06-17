from tensorflow import keras
from keras import Model, Input
from keras.layers import Dense
from glob import glob
from copy import deepcopy as copy
from keras.models import load_model
from constants import *


def encoder():
    inp = Input(AUDIO_LEN)
    if len(glob("model_enc_*")) != 0:
        print("loading encoder")
        choose = glob("model_enc_*")
        choose.sort()
        choose = choose[-1]
        enc = load_model(choose)(inp)
        return Model(inp, enc)

    print("generating encoder")
    for x in HIDDEN_LAYERS[:-1]:
        if "enc" in vars():
            enc = Dense(x, activation='elu')(enc)
        else:
            enc = Dense(x, activation='elu')(inp)
    if "enc" in vars():
        enc = Dense(HIDDEN_LAYERS[-1], activation="linear", name="bnek")(enc)
    else:
        enc = Dense(HIDDEN_LAYERS[-1], activation="linear", name="bnek")(inp)
    model = Model(inp, enc)
    model.summary()
    return model


def decoder():
    inp = Input(HIDDEN_LAYERS[-1])
    if len(glob("model_dec_*")) != 0:
        print("loading decoder")
        choose = glob("model_dec_*")
        choose.sort()
        choose = choose[-1]
        dec = load_model(choose)(inp)
        return Model(inp, dec)

    print("generating decoder")
    hl_copy = copy(HIDDEN_LAYERS)
    hl_copy.reverse()
    for x in hl_copy:
        if "dec" in vars():
            dec = Dense(x, activation='elu')(dec)
        else:
            dec = Dense(x, activation='elu')(inp)
    dec = Dense(AUDIO_LEN, activation='sigmoid')(dec)
    model = Model(inp, dec)
    model.summary()
    return model


def model_build(enc, dec):
    inp = Input(AUDIO_LEN)
    enc = enc(inp)
    dec = dec(enc)
    model = Model(inp, dec)
    model.compile(
        optimizer=keras.optimizers.Adadelta(1),
        loss="mean_absolute_percentage_error",
    )
    model.summary()
    return model
