from constants import *
from glob import glob
from random import shuffle
import numpy as np
import soundfile as sf
from model import model_build, encoder, decoder
from keras import Model, Input
from copy import deepcopy as copy

USE_FILES = 5

files = glob("./samples/*.npy")
files.sort()
shuffle(files)
files = files[:USE_FILES]

encoder = encoder()
decoder = decoder()
autoenc = model_build(encoder, decoder)

i = Input(AUDIO_LEN)
e = encoder(i)
enc_show = Model(i, e)
enc_show.compile("adam", loss="mean_absolute_error")

lis = []
for x in files:
    lis += [np.load(x)]
lis = np.asarray(lis).reshape(USE_FILES, AUDIO_LEN, 1)
for e, sl in enumerate(lis):
    sf.write(f"x_{e}.wav", (sl.reshape(AUDIO_LEN) * 2) - 1, SAMPLING)
out = autoenc.predict(lis, batch_size=1)
[print(f"y_{e}.wav:\n{x}\n") for e, x in enumerate(enc_show.predict(lis, batch_size=1))]
for e, sl in enumerate(out):
    sf.write(f"y_{e}.wav", (sl.reshape(AUDIO_LEN) * 2) - 1, SAMPLING)
